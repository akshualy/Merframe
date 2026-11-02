use anyhow::{Context, anyhow};
use keyring_core::api::CredentialStoreApi;
use keyring_core::{Entry, Error};

const SERVICE: &str = "net.yareli.merframe";
const ACCOUNT: &str = "warframe.market";

pub fn entry() -> anyhow::Result<Entry> {
    entry_for(SERVICE, ACCOUNT)
}

fn entry_for(service: &str, account: &str) -> anyhow::Result<Entry> {
    #[cfg(windows)]
    let store = windows_native_keyring_store::Store::new();
    #[cfg(target_os = "linux")]
    let store = dbus_secret_service_keyring_store::Store::new();
    #[cfg(any(windows, target_os = "linux"))]
    {
        let store = store.map_err(|_| unavailable())?;
        // Build directly against the native store: never use the library's mock
        // or sample store as a production fallback.
        #[cfg(windows)]
        let modifiers = Some(std::collections::HashMap::from([("persistence", "Local")]));
        #[cfg(target_os = "linux")]
        let modifiers = None;
        store
            .build(service, account, modifiers.as_ref())
            .map_err(|_| unavailable())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    Err(unavailable())
}

fn unavailable() -> anyhow::Error {
    // Some backend errors contain secret bytes; do not propagate their Debug
    // values or cause chains to logs or the frontend.
    anyhow!(
        "The OS credential store is unavailable. Unlock Windows Credential Manager or your Linux Secret Service and try again."
    )
}

pub fn read(entry: &Entry) -> anyhow::Result<Option<String>> {
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(Error::NoEntry) => Ok(None),
        Err(_) => Err(unavailable()),
    }
}

pub fn write(entry: &Entry, token: &str) -> anyhow::Result<()> {
    entry.set_password(token).map_err(|_| unavailable())?;
    if read(entry)?.as_deref() != Some(token) {
        return Err(anyhow!(
            "The OS credential store did not retain the session"
        ));
    }
    Ok(())
}

pub fn delete(entry: &Entry) -> anyhow::Result<()> {
    match entry.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(_) => Err(unavailable()),
    }
}

pub fn clear(
    entry: impl FnOnce() -> anyhow::Result<Entry>,
    cleanup: impl FnOnce() -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    // Attempt both removals even if a legacy file cannot be removed or the
    // credential store cannot be opened. Return an error so logout can be retried.
    let legacy = cleanup();
    let secure = entry().and_then(|entry| delete(&entry));
    legacy.and(secure)
}

pub fn migrate(
    entry: &Entry,
    legacy: impl FnOnce() -> anyhow::Result<Option<String>>,
    cleanup: impl FnOnce() -> anyhow::Result<()>,
) -> anyhow::Result<Option<String>> {
    if let Some(token) = read(entry)? {
        cleanup().context("Removing obsolete plaintext session copies")?;
        return Ok(Some(token));
    }
    let Some(token) = legacy()? else {
        return Ok(None);
    };
    write(entry, &token)?;
    // Only remove the old copy after a verified successful write. A locked
    // keyring must never silently discard an existing session.
    cleanup().context("Removing the migrated plaintext session")?;
    Ok(Some(token))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use keyring_core::api::CredentialApi;

    use super::*;

    fn mock_entry() -> Entry {
        keyring_core::mock::Store::new()
            .unwrap()
            .build(SERVICE, ACCOUNT, None)
            .unwrap()
    }

    fn fail_next(entry: &Entry) {
        entry
            .as_any()
            .downcast_ref::<keyring_core::mock::Cred>()
            .unwrap()
            .set_error(Error::BadEncoding(b"secret-do-not-log".to_vec()));
    }

    fn assert_redacted(error: &anyhow::Error) {
        assert!(!format!("{error:#}").contains("secret-do-not-log"));
        assert!(!format!("{error:?}").contains("secret-do-not-log"));
    }

    // A backend which acknowledges writes but cannot read the saved value.
    struct UnretainedCredential {
        written: AtomicBool,
        readback: fn() -> keyring_core::Result<Vec<u8>>,
    }

    impl CredentialApi for UnretainedCredential {
        fn set_secret(&self, _: &[u8]) -> keyring_core::Result<()> {
            self.written.store(true, Ordering::Relaxed);
            Ok(())
        }

        fn get_secret(&self) -> keyring_core::Result<Vec<u8>> {
            if self.written.load(Ordering::Relaxed) {
                (self.readback)()
            } else {
                Err(Error::NoEntry)
            }
        }

        fn delete_credential(&self) -> keyring_core::Result<()> {
            self.written.store(false, Ordering::Relaxed);
            Ok(())
        }

        fn get_credential(&self) -> keyring_core::Result<Option<Arc<keyring_core::Credential>>> {
            Ok(None)
        }

        fn get_specifiers(&self) -> Option<(String, String)> {
            None
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn migration_persists_before_removing_plaintext() {
        let entry = mock_entry();
        let cleaned = Cell::new(false);
        let token = migrate(
            &entry,
            || Ok(Some("legacy-session".to_owned())),
            || {
                assert_eq!(read(&entry)?.as_deref(), Some("legacy-session"));
                cleaned.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(token.as_deref(), Some("legacy-session"));
        assert!(cleaned.get());
        delete(&entry).unwrap();
        assert_eq!(read(&entry).unwrap(), None);
        delete(&entry).unwrap();
    }

    #[test]
    fn existing_secure_session_wins_and_stale_copies_are_removed() {
        let entry = mock_entry();
        write(&entry, "current-session").unwrap();
        let cleaned = Cell::new(false);
        let token = migrate(
            &entry,
            || panic!("An obsolete plaintext session must not be read"),
            || {
                cleaned.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(token.as_deref(), Some("current-session"));
        assert!(cleaned.get());
    }

    #[test]
    fn failed_write_preserves_the_legacy_copy() {
        let entry = mock_entry();
        let cleaned = Cell::new(false);
        let result = migrate(
            &entry,
            || {
                fail_next(&entry);
                Ok(Some("legacy-session".to_owned()))
            },
            || {
                cleaned.set(true);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!cleaned.get());
        assert_redacted(&result.unwrap_err());
        assert_eq!(read(&entry).unwrap(), None);
    }

    #[test]
    fn unverified_write_preserves_the_legacy_copy() {
        let readbacks: [fn() -> keyring_core::Result<Vec<u8>>; 3] = [
            || Err(Error::NoEntry),
            || Ok(b"unexpected-session".to_vec()),
            || Err(Error::BadEncoding(b"secret-do-not-log".to_vec())),
        ];
        for readback in readbacks {
            let entry = Entry::new_with_credential(Arc::new(UnretainedCredential {
                written: AtomicBool::new(false),
                readback,
            }));
            let error = migrate(
                &entry,
                || Ok(Some("legacy-session".to_owned())),
                || panic!("The legacy copy must survive a failed verification"),
            )
            .unwrap_err();
            assert_redacted(&error);
        }
    }

    #[test]
    fn failed_read_does_not_access_or_remove_legacy_session() {
        let entry = mock_entry();
        fail_next(&entry);
        let error = migrate(
            &entry,
            || panic!("Do not fall back to plaintext while the store is unavailable"),
            || panic!("Do not discard an unmigrated session"),
        )
        .unwrap_err();
        assert_redacted(&error);
    }

    #[test]
    fn logout_removes_secure_session_despite_failed_legacy_cleanup() {
        let store = keyring_core::mock::Store::new().unwrap();
        let entry = store.build(SERVICE, ACCOUNT, None).unwrap();
        write(&entry, "session").unwrap();
        assert!(
            clear(
                || store.build(SERVICE, ACCOUNT, None).map_err(Into::into),
                || Err(anyhow!("simulated file permission failure")),
            )
            .is_err()
        );
        assert_eq!(read(&entry).unwrap(), None);
    }

    #[test]
    fn logout_cleans_legacy_copies_when_store_cannot_be_opened() {
        let cleaned = Cell::new(false);
        assert!(
            clear(
                || Err(unavailable()),
                || {
                    cleaned.set(true);
                    Ok(())
                },
            )
            .is_err()
        );
        assert!(cleaned.get());
    }

    #[test]
    fn failed_secure_logout_is_redacted_and_can_be_retried() {
        let store = keyring_core::mock::Store::new().unwrap();
        let entry = store.build(SERVICE, ACCOUNT, None).unwrap();
        write(&entry, "session").unwrap();
        fail_next(&entry);
        let error = clear(
            || store.build(SERVICE, ACCOUNT, None).map_err(Into::into),
            || Ok(()),
        )
        .unwrap_err();
        assert_redacted(&error);
        assert_eq!(read(&entry).unwrap().as_deref(), Some("session"));
        clear(
            || store.build(SERVICE, ACCOUNT, None).map_err(Into::into),
            || Ok(()),
        )
        .unwrap();
        assert_eq!(read(&entry).unwrap(), None);
    }

    #[test]
    #[ignore = "Requires the current user's unlocked native credential store"]
    fn native_credential_store_round_trip() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let service = "net.yareli.merframe.integration-test";
        let account = unique.to_string();
        let entry = entry_for(service, &account).unwrap();
        struct Cleanup(Entry);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = delete(&self.0);
            }
        }
        let _cleanup = Cleanup(entry_for(service, &account).unwrap());
        let migrated = migrate(
            &entry,
            || Ok(Some("temporary-test-session".to_owned())),
            || {
                let reopened = entry_for(service, &account)?;
                assert_eq!(read(&reopened)?.as_deref(), Some("temporary-test-session"));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(migrated.as_deref(), Some("temporary-test-session"));
        write(&entry, "rotated-test-session").unwrap();
        assert_eq!(
            read(&entry_for(service, &account).unwrap())
                .unwrap()
                .as_deref(),
            Some("rotated-test-session")
        );
        clear(|| entry_for(service, &account), || Ok(())).unwrap();
        assert_eq!(read(&entry).unwrap(), None);
        clear(|| entry_for(service, &account), || Ok(())).unwrap();
    }

    #[test]
    fn cleanup_failure_can_be_retried_without_overwriting_secure_session() {
        let entry = mock_entry();
        assert!(
            migrate(
                &entry,
                || Ok(Some("session".to_owned())),
                || Err(anyhow!("simulated file permission failure")),
            )
            .is_err()
        );
        assert_eq!(read(&entry).unwrap().as_deref(), Some("session"));
        assert_eq!(
            migrate(&entry, || Ok(None), || Ok(())).unwrap().as_deref(),
            Some("session")
        );
    }
}
