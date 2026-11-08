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
        // Build directly against the native store, never the library's mock store.
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
    // Backend errors can contain secret bytes. Do not propagate their causes.
    anyhow!("The OS credential store is unavailable")
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

pub trait TokenFiles {
    fn read(&self) -> anyhow::Result<Option<String>>;
    fn legacy(&self) -> anyhow::Result<Option<String>>;
    fn write(&self, token: &str) -> anyhow::Result<()>;
    fn remove(&self) -> anyhow::Result<()>;
}

fn migrate(entry: &Entry, token: &str, files: &impl TokenFiles) {
    match write(entry, token) {
        Ok(()) => {
            if let Err(error) = files.remove() {
                tracing::warn!(%error, "Plaintext market session cleanup will be retried");
            }
        }
        Err(error) => {
            tracing::warn!(%error, "Keeping the plaintext market session");
        }
    }
}

pub fn load(
    entry: anyhow::Result<Entry>,
    files: &impl TokenFiles,
) -> anyhow::Result<Option<String>> {
    // A fallback file may hold a newer login or rotation than the secure store.
    if let Some(token) = files.read()? {
        if token.is_empty() {
            // An empty file records logout while secure storage was unavailable.
            // Keep it until all older copies are removed, preventing resurrection.
            if entry.and_then(|entry| delete(&entry)).is_ok()
                && let Err(error) = files.remove()
            {
                tracing::warn!(%error, "Logged-out market session cleanup will be retried");
            }
            return Ok(None);
        }
        if let Ok(entry) = entry {
            migrate(&entry, &token, files);
        }
        return Ok(Some(token));
    }

    let entry = match entry {
        Ok(entry) => entry,
        Err(error) => {
            tracing::warn!(%error, "Loading the plaintext market session");
            return files.legacy();
        }
    };
    match read(&entry) {
        Ok(Some(token)) => {
            if let Err(error) = files.remove() {
                tracing::warn!(%error, "Plaintext market session cleanup will be retried");
            }
            Ok(Some(token))
        }
        Ok(None) => {
            let token = files.legacy()?;
            if let Some(token) = &token {
                migrate(&entry, token, files);
            }
            Ok(token)
        }
        Err(error) => {
            // Do not overwrite an unreadable secure session with obsolete JSON.
            tracing::warn!(%error, "Loading the plaintext market session");
            files.legacy()
        }
    }
}

pub fn save(
    entry: anyhow::Result<Entry>,
    token: &str,
    files: &impl TokenFiles,
) -> anyhow::Result<()> {
    if token.is_empty() {
        return Err(anyhow!("The market session token is empty"));
    }
    match entry.and_then(|entry| write(&entry, token)) {
        Ok(()) => {
            if let Err(error) = files.remove() {
                // An older fallback must never shadow the newly saved token.
                files.write(token)?;
                tracing::warn!(%error, "Keeping the current market session in plaintext");
            }
            Ok(())
        }
        Err(error) => {
            files
                .write(token)
                .context("Saving the plaintext market session")?;
            tracing::warn!(%error, "Saved the market session in plaintext");
            Ok(())
        }
    }
}

pub fn clear(entry: anyhow::Result<Entry>, files: &impl TokenFiles) -> anyhow::Result<()> {
    // Record logout before deleting anything. If the secure store is locked,
    // this marker suppresses its stale session until deletion can be retried.
    let marker = files.write("");
    match entry.and_then(|entry| delete(&entry)) {
        Ok(()) => files.remove(),
        Err(error) => {
            marker.context("Recording market logout")?;
            tracing::warn!(%error, "Secure market session deletion will be retried");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use keyring_core::api::CredentialApi;

    use super::*;

    #[derive(Default)]
    struct Files {
        token: RefCell<Option<String>>,
        legacy: RefCell<Option<String>>,
        fail_write: Cell<bool>,
        fail_cleanup: Cell<bool>,
        writes: Cell<usize>,
    }

    impl TokenFiles for Files {
        fn read(&self) -> anyhow::Result<Option<String>> {
            Ok(self.token.borrow().clone())
        }

        fn legacy(&self) -> anyhow::Result<Option<String>> {
            Ok(self.legacy.borrow().clone())
        }

        fn write(&self, token: &str) -> anyhow::Result<()> {
            if self.fail_write.get() {
                return Err(anyhow!("simulated file write failure"));
            }
            self.writes.set(self.writes.get() + 1);
            self.token.replace(Some(token.to_owned()));
            Ok(())
        }

        fn remove(&self) -> anyhow::Result<()> {
            if self.fail_cleanup.get() {
                return Err(anyhow!("simulated file cleanup failure"));
            }
            self.legacy.replace(None);
            self.token.replace(None);
            Ok(())
        }
    }

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
    fn healthy_store_does_not_write_plaintext() {
        let entry = mock_entry();
        let files = Files::default();
        save(Ok(entry), "session", &files).unwrap();
        assert_eq!(files.read().unwrap(), None);
        assert_eq!(files.writes.get(), 0);
    }

    #[test]
    fn unavailable_store_persists_and_loads_plaintext() {
        let files = Files::default();
        save(Err(unavailable()), "session", &files).unwrap();
        assert_eq!(files.read().unwrap().as_deref(), Some("session"));
        assert_eq!(
            load(Err(unavailable()), &files).unwrap().as_deref(),
            Some("session")
        );
    }

    #[test]
    fn fallback_overrides_stale_secure_session_and_migrates_on_recovery() {
        let store = keyring_core::mock::Store::new().unwrap();
        let entry = store.build(SERVICE, ACCOUNT, None).unwrap();
        let files = Files::default();
        write(&entry, "old-session").unwrap();
        save(Err(unavailable()), "new-session", &files).unwrap();
        assert_eq!(
            load(Ok(store.build(SERVICE, ACCOUNT, None).unwrap()), &files)
                .unwrap()
                .as_deref(),
            Some("new-session")
        );
        assert_eq!(read(&entry).unwrap().as_deref(), Some("new-session"));
        assert_eq!(files.read().unwrap(), None);
    }

    #[test]
    fn existing_secure_session_wins_over_legacy_json() {
        let entry = mock_entry();
        write(&entry, "current-session").unwrap();
        let files = Files::default();
        files.legacy.replace(Some("obsolete-session".to_owned()));
        assert_eq!(
            load(Ok(entry), &files).unwrap().as_deref(),
            Some("current-session")
        );
        assert_eq!(files.legacy().unwrap(), None);
    }

    #[test]
    fn legacy_json_stays_usable_when_secure_read_fails() {
        let entry = mock_entry();
        write(&entry, "current-secure-session").unwrap();
        fail_next(&entry);
        let files = Files::default();
        files.legacy.replace(Some("legacy-session".to_owned()));
        assert_eq!(
            load(Ok(entry), &files).unwrap().as_deref(),
            Some("legacy-session")
        );
        assert_eq!(files.legacy().unwrap().as_deref(), Some("legacy-session"));
    }

    #[test]
    fn failed_or_unverified_write_uses_plaintext() {
        let entry = mock_entry();
        fail_next(&entry);
        let files = Files::default();
        save(Ok(entry), "session", &files).unwrap();
        assert_eq!(files.read().unwrap().as_deref(), Some("session"));

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
            let files = Files::default();
            save(Ok(entry), "session", &files).unwrap();
            assert_eq!(files.read().unwrap().as_deref(), Some("session"));
        }
    }

    #[test]
    fn failed_migration_keeps_plaintext_login_usable() {
        let entry = mock_entry();
        fail_next(&entry);
        let files = Files::default();
        files.write("session").unwrap();
        assert_eq!(load(Ok(entry), &files).unwrap().as_deref(), Some("session"));
        assert_eq!(files.read().unwrap().as_deref(), Some("session"));
    }

    #[test]
    fn failed_cleanup_never_restores_an_older_fallback() {
        let store = keyring_core::mock::Store::new().unwrap();
        let files = Files::default();
        files.write("old-session").unwrap();
        files.fail_cleanup.set(true);
        save(
            Ok(store.build(SERVICE, ACCOUNT, None).unwrap()),
            "new-session",
            &files,
        )
        .unwrap();
        assert_eq!(files.read().unwrap().as_deref(), Some("new-session"));
        files.fail_cleanup.set(false);
        assert_eq!(
            load(Ok(store.build(SERVICE, ACCOUNT, None).unwrap()), &files)
                .unwrap()
                .as_deref(),
            Some("new-session")
        );
        assert_eq!(files.read().unwrap(), None);
    }

    #[test]
    fn failed_plaintext_write_reports_persistence_error() {
        let files = Files::default();
        files.fail_write.set(true);
        assert!(save(Err(unavailable()), "session", &files).is_err());
        assert_eq!(files.read().unwrap(), None);
    }

    #[test]
    fn logout_while_locked_survives_restart_and_cleans_up_on_recovery() {
        let store = keyring_core::mock::Store::new().unwrap();
        let entry = store.build(SERVICE, ACCOUNT, None).unwrap();
        let files = Files::default();
        write(&entry, "old-secure-session").unwrap();
        files.write("fallback-session").unwrap();
        files.legacy.replace(Some("old-json-session".to_owned()));
        clear(Err(unavailable()), &files).unwrap();
        assert_eq!(files.read().unwrap().as_deref(), Some(""));
        assert_eq!(load(Err(unavailable()), &files).unwrap(), None);
        assert_eq!(
            load(Ok(store.build(SERVICE, ACCOUNT, None).unwrap()), &files).unwrap(),
            None
        );
        assert_eq!(read(&entry).unwrap(), None);
        assert_eq!(files.read().unwrap(), None);
        assert_eq!(files.legacy().unwrap(), None);
    }

    #[test]
    fn failed_logout_cleanup_keeps_marker_until_retry() {
        let files = Files::default();
        files.legacy.replace(Some("old-json-session".to_owned()));
        files.fail_cleanup.set(true);
        assert!(clear(Ok(mock_entry()), &files).is_err());
        assert_eq!(load(Ok(mock_entry()), &files).unwrap(), None);
        assert_eq!(files.read().unwrap().as_deref(), Some(""));
        files.fail_cleanup.set(false);
        assert_eq!(load(Ok(mock_entry()), &files).unwrap(), None);
        assert_eq!(files.read().unwrap(), None);
        assert_eq!(files.legacy().unwrap(), None);
    }

    #[test]
    fn unavailable_store_and_failed_marker_write_report_logout_error() {
        let files = Files::default();
        files.fail_write.set(true);
        assert!(clear(Err(unavailable()), &files).is_err());
    }

    #[test]
    fn backend_errors_remain_redacted() {
        let entry = mock_entry();
        fail_next(&entry);
        assert_redacted(&read(&entry).unwrap_err());
        fail_next(&entry);
        assert_redacted(&write(&entry, "session").unwrap_err());
        fail_next(&entry);
        assert_redacted(&delete(&entry).unwrap_err());
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
        let files = Files::default();
        files.write("temporary-test-session").unwrap();
        assert_eq!(
            load(entry_for(service, &account), &files)
                .unwrap()
                .as_deref(),
            Some("temporary-test-session")
        );
        assert_eq!(files.read().unwrap(), None);
        assert_eq!(
            read(&entry).unwrap().as_deref(),
            Some("temporary-test-session")
        );
        save(entry_for(service, &account), "rotated-test-session", &files).unwrap();
        assert_eq!(
            read(&entry).unwrap().as_deref(),
            Some("rotated-test-session")
        );
        clear(entry_for(service, &account), &files).unwrap();
        assert_eq!(read(&entry).unwrap(), None);
        assert_eq!(files.read().unwrap(), None);
        clear(entry_for(service, &account), &files).unwrap();
    }
}
