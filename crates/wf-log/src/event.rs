use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use crate::line::LogLine;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    LoggedIn {
        name: String,
    },
    LoggingIn,
    MissionSet {
        node: String,
        void_tier: Option<String>,
    },
    MissionStart {
        level_path: String,
    },
    MissionSucceeded,
    MissionRewardsGiven,
    ReturnedToShip,
    RelicRewardScreenOpened,
    OwnRelicReward {
        account_id: String,
        store_item: String,
    },
    SquadRewardInfoReceived {
        account_id: String,
    },
    SquadRewardInfoComplete,
    RelicRewardsShown,
    RelicSelectScreenLoaded,
    StarChartOpened,
    StarChartHidden,
    MissionCleared,
    SquadMissionPending {
        node: String,
    },
    TradeScreen {
        visible: bool,
    },
    DucatKiosk {
        visible: bool,
    },
    DucatSale,
    ConfirmDialog {
        buttons: DialogButtons,
    },
    DialogAnswered {
        accepted: bool,
    },
    ChatTabAdded {
        channel: String,
    },
    RivenDialog {
        item_path: String,
    },
    PurchaseDialogHudVisibility {
        count: u32,
    },
    RivenRerollScreenLoaded,
    SceneTornDown,
    ConsoleOpened,
    InputMappingReset,
    InventoryCommitted,
    InventorySynced,
    WindowFocus {
        focused: bool,
    },
    GameMonitor(MonitorRect),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogButtons {
    OkCancel,
    YesNo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorRect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

impl Event {
    pub fn label(&self) -> &'static str {
        match self {
            Self::LoggedIn { .. } => "LoggedIn",
            Self::LoggingIn => "LoggingIn",
            Self::MissionSet { .. } => "MissionSet",
            Self::MissionStart { .. } => "MissionStart",
            Self::MissionSucceeded => "MissionSucceeded",
            Self::MissionRewardsGiven => "MissionRewardsGiven",
            Self::ReturnedToShip => "ReturnedToShip",
            Self::RelicRewardScreenOpened => "RelicRewardScreenOpened",
            Self::OwnRelicReward { .. } => "OwnRelicReward",
            Self::SquadRewardInfoReceived { .. } => "SquadRewardInfoReceived",
            Self::SquadRewardInfoComplete => "SquadRewardInfoComplete",
            Self::RelicRewardsShown => "RelicRewardsShown",
            Self::RelicSelectScreenLoaded => "RelicSelectScreenLoaded",
            Self::StarChartOpened => "StarChartOpened",
            Self::StarChartHidden => "StarChartHidden",
            Self::MissionCleared => "MissionCleared",
            Self::SquadMissionPending { .. } => "SquadMissionPending",
            Self::TradeScreen { .. } => "TradeScreen",
            Self::DucatKiosk { .. } => "DucatKiosk",
            Self::DucatSale => "DucatSale",
            Self::ConfirmDialog { .. } => "ConfirmDialog",
            Self::DialogAnswered { .. } => "DialogAnswered",
            Self::ChatTabAdded { .. } => "ChatTabAdded",
            Self::RivenDialog { .. } => "RivenDialog",
            Self::PurchaseDialogHudVisibility { .. } => "PurchaseDialogHudVisibility",
            Self::RivenRerollScreenLoaded => "RivenRerollScreenLoaded",
            Self::SceneTornDown => "SceneTornDown",
            Self::ConsoleOpened => "ConsoleOpened",
            Self::InputMappingReset => "InputMappingReset",
            Self::InventoryCommitted => "InventoryCommitted",
            Self::InventorySynced => "InventorySynced",
            Self::WindowFocus { .. } => "WindowFocus",
            Self::GameMonitor(_) => "GameMonitor",
        }
    }
}

fn compiled(pattern: &str) -> Regex {
    match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(_) => unreachable!(),
    }
}

static LOGGED_IN: LazyLock<Regex> = LazyLock::new(|| compiled(r"Logged in\s+([^\s(]+)"));
static OWN_RELIC_REWARD: LazyLock<Regex> =
    LazyLock::new(|| compiled(r"VoidProjections: ([0-9a-f]{24}) gets reward (\S+)$"));
static SQUAD_REWARD_INFO_RECEIVED: LazyLock<Regex> = LazyLock::new(|| {
    compiled(r"VoidProjections: (?:Client|Host) got reward info from ([0-9a-f]{24})$")
});
static CHAT_TAB_ADDED: LazyLock<Regex> =
    LazyLock::new(|| compiled(r"Adding tab with channel name: (\S+) to index"));
static RIVEN_DIALOG: LazyLock<Regex> =
    LazyLock::new(|| compiled(r"ThemedDetailedPurchaseDialog\.lua: PopulateInfo->(\S+)$"));
static PURCHASE_DIALOG_HUD_VIS: LazyLock<Regex> =
    LazyLock::new(|| compiled(r"ThemedDetailedPurchaseDialog\.lua: DBG: HudVis (\d+)$"));
static MONITOR_INFO: LazyLock<Regex> =
    LazyLock::new(|| compiled(r"^Monitor Info \((-?\d+), (-?\d+), (-?\d+), (-?\d+)\)\.$"));

#[derive(Deserialize)]
struct MissionSetWire {
    name: String,
    #[serde(rename = "voidTier")]
    void_tier: Option<String>,
}

fn mission_set(message: &str) -> Option<Event> {
    let json_part = message.strip_prefix("Set squad mission:")?.trim();
    if json_part.is_empty() {
        return Some(Event::MissionCleared);
    }
    let wire: MissionSetWire = serde_json::from_str(json_part).ok()?;
    Some(Event::MissionSet {
        node: wire.name,
        void_tier: wire.void_tier,
    })
}

fn host_start_match_path(message: &str) -> Option<String> {
    let (_, after) = message.split_once("launching level for ")?;
    let (_, inside) = after.split_once('(')?;
    let close = inside.rfind(')')?;
    Some(inside[..close].to_owned())
}

fn open_level_path(message: &str) -> Option<String> {
    let path = message.strip_prefix("FrameworkCmd::OpenLevel - ")?;
    if path.contains("PlayerShip") {
        return None;
    }
    Some(path.to_owned())
}

fn dialog(call: &str) -> Option<Event> {
    match call {
        "SendResult(4)" => Some(Event::DialogAnswered { accepted: true }),
        "SendResult(5)" => Some(Event::DialogAnswered { accepted: false }),
        _ => confirm_dialog(call),
    }
}

fn confirm_dialog(call: &str) -> Option<Event> {
    let (created, buttons) = call.rsplit_once(" leftItem=")?;
    if !created.starts_with("CreateOkCancel(") {
        return None;
    }
    let buttons = match buttons {
        "/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)" => DialogButtons::OkCancel,
        "/Menu/Confirm_Item_Yes, rightItem=/Menu/Confirm_Item_No)" => DialogButtons::YesNo,
        _ => return None,
    };
    Some(Event::ConfirmDialog { buttons })
}

fn game_monitor(message: &str) -> Option<Event> {
    let captures = MONITOR_INFO.captures(message)?;
    let edge = |index: usize| captures[index].parse::<i32>().ok();
    let (left, right, top, bottom) = (edge(1)?, edge(2)?, edge(3)?, edge(4)?);
    let width = u32::try_from(right - left).ok()?;
    let height = u32::try_from(bottom - top).ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    Some(Event::GameMonitor(MonitorRect {
        left,
        top,
        width,
        height,
    }))
}

fn interface_state(message: &str) -> Option<Event> {
    if let Some(state) = message.strip_prefix("WM_ACTIVATEAPP ") {
        return match state {
            "0" => Some(Event::WindowFocus { focused: false }),
            "1" => Some(Event::WindowFocus { focused: true }),
            _ => None,
        };
    }
    if message.contains("ytes of recycled effects")
        || message.contains("NpcManager::ClearAgents() ReadyToCreateAgents = false")
    {
        return Some(Event::SceneTornDown);
    }
    if message.contains("UIConsoleTrigger::Open()") {
        return Some(Event::ConsoleOpened);
    }
    if message.contains("InitMapping for all devices with bindings") {
        return Some(Event::InputMappingReset);
    }
    if message
        == "Subscribing for /Lotus/Interface/MapRedux.swf with input filter /EE/Types/Input/MapReduxInputFilter"
    {
        return Some(Event::StarChartOpened);
    }
    if message == "MapRedux.lua: DBG: HudVis 0" {
        return Some(Event::StarChartHidden);
    }
    None
}

fn relic_rewards(message: &str) -> Option<Event> {
    if message.starts_with("VoidProjections: OpenVoidProjectionRewardScreen") {
        return Some(Event::RelicRewardScreenOpened);
    }
    if let Some(captures) = OWN_RELIC_REWARD.captures(message) {
        return Some(Event::OwnRelicReward {
            account_id: captures[1].to_owned(),
            store_item: captures[2].to_owned(),
        });
    }
    if let Some(captures) = SQUAD_REWARD_INFO_RECEIVED.captures(message) {
        return Some(Event::SquadRewardInfoReceived {
            account_id: captures[1].to_owned(),
        });
    }
    if message.ends_with("has reward info for all players now") {
        return Some(Event::SquadRewardInfoComplete);
    }
    if message.ends_with("ProjectionRewardChoice.lua: Got rewards") {
        return Some(Event::RelicRewardsShown);
    }
    None
}

fn purchase_dialog(message: &str) -> Option<Event> {
    if let Some(captures) = PURCHASE_DIALOG_HUD_VIS.captures(message) {
        return captures[1]
            .parse()
            .ok()
            .map(|count| Event::PurchaseDialogHudVisibility { count });
    }
    if let Some(captures) = RIVEN_DIALOG.captures(message) {
        return Some(Event::RivenDialog {
            item_path: captures[1].to_owned(),
        });
    }
    None
}

pub fn classify(line: &LogLine) -> Option<Event> {
    let message = line.message.as_str();

    if let Some(captures) = LOGGED_IN.captures(message) {
        return Some(Event::LoggedIn {
            name: captures[1].to_owned(),
        });
    }
    if message.starts_with("Logging in as") {
        return Some(Event::LoggingIn);
    }
    if message.starts_with("Set squad mission: ") {
        return mission_set(message);
    }
    if let Some(node) = message.strip_prefix("ThemedSquadOverlay.lua: Pending mission: ") {
        return Some(Event::SquadMissionPending {
            node: node.to_owned(),
        });
    }
    if message.contains("Lobby::Host_StartMatch: launching level for") {
        return host_start_match_path(message).map(|level_path| Event::MissionStart { level_path });
    }
    if message.starts_with("FrameworkCmd::OpenLevel - ") {
        return open_level_path(message).map(|level_path| Event::MissionStart { level_path });
    }
    if let Some(call) = message.strip_prefix("Dialog.lua: Dialog::") {
        return dialog(call);
    }
    if message.ends_with("OmegaRerollSelection.lua: Diorama setup") {
        return Some(Event::RivenRerollScreenLoaded);
    }
    match message.strip_prefix("Trade.lua: DBG: HudVis ") {
        Some("0") => return Some(Event::TradeScreen { visible: false }),
        Some("1") => return Some(Event::TradeScreen { visible: true }),
        _ => {}
    }
    if message == "InventoryTest.lua: InventoryTest - CurrMode: Selling Prime Parts" {
        return Some(Event::DucatKiosk { visible: true });
    }
    if message == "InventoryTest.lua: OnSellCompleted(result=true, body={})" {
        return Some(Event::DucatSale);
    }
    if message == "InventoryTest.lua: DBG: HudVis 0" {
        return Some(Event::DucatKiosk { visible: false });
    }
    if message.ends_with("Mission Succeeded") {
        return Some(Event::MissionSucceeded);
    }
    if message.ends_with("GiveMissionRewards. success=true") {
        return Some(Event::MissionRewardsGiven);
    }
    if message.ends_with("ReturnedToShip") {
        return Some(Event::ReturnedToShip);
    }
    if let Some(event) = relic_rewards(message) {
        return Some(event);
    }
    if message.ends_with("ThemedProjectionManager.lua: LoadingCompleteEnd") {
        return Some(Event::RelicSelectScreenLoaded);
    }
    if let Some(captures) = CHAT_TAB_ADDED.captures(message) {
        return Some(Event::ChatTabAdded {
            channel: captures[1].to_owned(),
        });
    }
    if let Some(event) = purchase_dialog(message) {
        return Some(event);
    }
    if let Some(event) = interface_state(message) {
        return Some(event);
    }
    if let Some(event) = game_monitor(message) {
        return Some(event);
    }
    if message.ends_with("CommitInventoryChangesToDB") {
        return Some(Event::InventoryCommitted);
    }
    if message.ends_with("SyncInventoryFromDB")
        || message.ends_with("Inventory sync done")
        || message.starts_with("OnInventoryResults completed in")
    {
        return Some(Event::InventorySynced);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::line::{Channel, Level};

    fn log_line(message: &str) -> LogLine {
        LogLine {
            time: 0.0,
            channel: Channel::Sys,
            level: Level::Info,
            message: message.to_owned(),
        }
    }

    #[test]
    fn logged_in() {
        let event = classify(&log_line("Logged in TestTenno"));
        assert_eq!(
            event,
            Some(Event::LoggedIn {
                name: String::from("TestTenno")
            })
        );
    }

    #[test]
    fn logging_in() {
        let event = classify(&log_line("Logging in as "));
        assert_eq!(event, Some(Event::LoggingIn));
    }

    #[test]
    fn mission_set_with_void_tier() {
        let event = classify(&log_line(
            r#"Set squad mission: {"difficulty":"","voidTier":"VoidT1","quest":"","name":"SolNode14_ActiveMission"}"#,
        ));
        assert_eq!(
            event,
            Some(Event::MissionSet {
                node: "SolNode14_ActiveMission".to_owned(),
                void_tier: Some("VoidT1".to_owned()),
            })
        );
    }

    #[test]
    fn squad_mission_pending() {
        let event = classify(&log_line(
            "ThemedSquadOverlay.lua: Pending mission: SolNode232_ActiveMission",
        ));
        assert_eq!(
            event,
            Some(Event::SquadMissionPending {
                node: "SolNode232_ActiveMission".to_owned(),
            })
        );
    }

    #[test]
    fn host_start_match() {
        let event = classify(&log_line(
            "ThemedSquadOverlay.lua: Lobby::Host_StartMatch: launching level for EventNode25_Event (/Lotus/Levels/GrineerBeach/GrineerBeachEvent.level)",
        ));
        assert_eq!(
            event,
            Some(Event::MissionStart {
                level_path: "/Lotus/Levels/GrineerBeach/GrineerBeachEvent.level".to_owned()
            })
        );
    }

    #[test]
    fn open_level_player_ship() {
        assert_eq!(
            classify(&log_line(
                "FrameworkCmd::OpenLevel - /Lotus/Levels/Proc/PlayerShip"
            )),
            None
        );
    }

    #[test]
    fn open_level_mission() {
        let event = classify(&log_line(
            "FrameworkCmd::OpenLevel - /Lotus/Levels/Proc/Corpus/CorpusIcePlanetExterminateCaves",
        ));
        assert_eq!(
            event,
            Some(Event::MissionStart {
                level_path: "/Lotus/Levels/Proc/Corpus/CorpusIcePlanetExterminateCaves".to_owned()
            })
        );
    }

    fn dialog_line(description: &str, buttons: &str) -> LogLine {
        log_line(&format!(
            "Dialog.lua: Dialog::CreateOkCancel(description={description}, title= {buttons})"
        ))
    }

    const OK_CANCEL: &str = "leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel";
    const YES_NO: &str = "leftItem=/Menu/Confirm_Item_Yes, rightItem=/Menu/Confirm_Item_No";

    #[test]
    fn confirmations_in_any_language() {
        for description in [
            "Are you sure you want to accept this trade? You are offering\nForma Blueprint x 2\nand will receive from TestSquadA\u{e000} the following:\nPlatinum x 45\n",
            "Bist du sicher, dass du diesen Handel annehmen willst? Du bietest:\nForma-Blaupause x 2\nund erhaeltst von TestSquadA Folgendes:\nPlatin x 45",
            "/Lotus/Language/Menu/ExitDojoConfirm",
        ] {
            assert_eq!(
                classify(&dialog_line(description, OK_CANCEL)),
                Some(Event::ConfirmDialog {
                    buttons: DialogButtons::OkCancel,
                })
            );
        }
        for description in [
            "Are you sure you want to cycle Kompressa Vexitis for \u{e071}3,500?",
            "Willst du Kompressa Vexitis wirklich fuer \u{e071}3.500 umwandeln?",
            "Cycle Riven into current selection?",
        ] {
            assert_eq!(
                classify(&dialog_line(description, YES_NO)),
                Some(Event::ConfirmDialog {
                    buttons: DialogButtons::YesNo,
                })
            );
        }
    }

    #[test]
    fn wait_dialogs_and_truncated_lines() {
        assert_eq!(
            classify(&dialog_line(
                "/Lotus/Language/Menu/NavBar_QuickMatchPleaseWait",
                "leftItem=nil, rightItem=nil"
            )),
            None
        );
        assert_eq!(
            classify(&log_line(
                "Dialog.lua: Dialog::CreateOkCancel(description=Bist du sicher, dass du diesen Handel annehmen willst? Du bietest:"
            )),
            None
        );
    }

    #[test]
    fn notice_dialogs_are_not_events() {
        assert_eq!(
            classify(&log_line(
                "Dialog.lua: Dialog::CreateOk(description=Der Handel war erfolgreich!, title= leftItem=/Menu/Confirm_Item_Ok)"
            )),
            None
        );
    }

    #[test]
    fn dialog_answers() {
        assert_eq!(
            classify(&log_line("Dialog.lua: Dialog::SendResult(4)")),
            Some(Event::DialogAnswered { accepted: true })
        );
        assert_eq!(
            classify(&log_line("Dialog.lua: Dialog::SendResult(5)")),
            Some(Event::DialogAnswered { accepted: false })
        );
        assert_eq!(
            classify(&log_line("Dialog.lua: Dialog::SendResult(3)")),
            None
        );
        assert_eq!(
            classify(&log_line("Dialog.lua: SendResult_MENU_SELECT()")),
            None
        );
    }

    #[test]
    fn trade_screen_visibility() {
        assert_eq!(
            classify(&log_line("Trade.lua: DBG: HudVis 1")),
            Some(Event::TradeScreen { visible: true })
        );
        assert_eq!(
            classify(&log_line("Trade.lua: DBG: HudVis 0")),
            Some(Event::TradeScreen { visible: false })
        );
    }

    #[test]
    fn ducat_sale() {
        assert_eq!(
            classify(&log_line(
                "InventoryTest.lua: OnSellCompleted(result=true, body={})"
            )),
            Some(Event::DucatSale)
        );
        assert_eq!(
            classify(&log_line(
                "InventoryTest.lua: OnSellCompleted(result=false, body={})"
            )),
            None
        );
    }

    #[test]
    fn ducat_kiosk_visibility() {
        assert_eq!(
            classify(&log_line(
                "InventoryTest.lua: InventoryTest - CurrMode: Selling Prime Parts"
            )),
            Some(Event::DucatKiosk { visible: true })
        );
        assert_eq!(
            classify(&log_line("InventoryTest.lua: DBG: HudVis 0")),
            Some(Event::DucatKiosk { visible: false })
        );
        assert_eq!(
            classify(&log_line(
                "InventoryTest.lua: InventoryTest - CurrMode: Inventory"
            )),
            None
        );
        assert_eq!(
            classify(&log_line("InventoryTest.lua: DBG: HudVis 1")),
            None
        );
    }

    #[test]
    fn mission_succeeded() {
        assert_eq!(
            classify(&log_line("EndOfMatch.lua: Mission Succeeded")),
            Some(Event::MissionSucceeded)
        );
    }

    #[test]
    fn mission_rewards_given() {
        assert_eq!(
            classify(&log_line(
                "EndOfMatch.lua: EndOfMatch.lua: GiveMissionRewards. success=true"
            )),
            Some(Event::MissionRewardsGiven)
        );
    }

    #[test]
    fn returned_to_ship() {
        assert_eq!(
            classify(&log_line("EndOfMatch.lua: ReturnedToShip")),
            Some(Event::ReturnedToShip)
        );
    }

    #[test]
    fn reward_screen_opened() {
        assert_eq!(
            classify(&log_line(
                "VoidProjections: OpenVoidProjectionRewardScreen - PostMigration: 0"
            )),
            Some(Event::RelicRewardScreenOpened)
        );
    }

    #[test]
    fn reward_screen_opened_rmi() {
        assert_eq!(
            classify(&log_line(
                "VoidProjections: OpenVoidProjectionRewardScreenRMI"
            )),
            Some(Event::RelicRewardScreenOpened)
        );
    }

    #[test]
    fn own_relic_reward() {
        let event = classify(&log_line(
            "VoidProjections: 5f0a1b2c3d4e5f6a7b8c9d0e gets reward /Lotus/StoreItems/Types/Recipes/Weapons/PrimeDaikyuBlueprint",
        ));
        assert_eq!(
            event,
            Some(Event::OwnRelicReward {
                account_id: "5f0a1b2c3d4e5f6a7b8c9d0e".to_owned(),
                store_item: "/Lotus/StoreItems/Types/Recipes/Weapons/PrimeDaikyuBlueprint"
                    .to_owned(),
            })
        );
    }

    #[test]
    fn squad_reward_info_received() {
        for role in ["Client", "Host"] {
            let event = classify(&log_line(&format!(
                "VoidProjections: {role} got reward info from 5f0a1b2c3d4e5f6a7b8c9d0e"
            )));
            assert_eq!(
                event,
                Some(Event::SquadRewardInfoReceived {
                    account_id: "5f0a1b2c3d4e5f6a7b8c9d0e".to_owned()
                })
            );
        }
    }

    #[test]
    fn squad_reward_info_complete() {
        for role in ["Client", "Host"] {
            assert_eq!(
                classify(&log_line(&format!(
                    "VoidProjections: {role} has reward info for all players now"
                ))),
                Some(Event::SquadRewardInfoComplete)
            );
        }
    }

    #[test]
    fn relic_rewards_shown() {
        assert_eq!(
            classify(&log_line("ProjectionRewardChoice.lua: Got rewards")),
            Some(Event::RelicRewardsShown)
        );
    }

    #[test]
    fn relic_select_screen_loaded() {
        assert_eq!(
            classify(&log_line("ThemedProjectionManager.lua: LoadingCompleteEnd")),
            Some(Event::RelicSelectScreenLoaded)
        );
    }

    #[test]
    fn star_chart_shown_and_hidden() {
        assert_eq!(
            classify(&log_line(
                "Subscribing for /Lotus/Interface/MapRedux.swf with input filter /EE/Types/Input/MapReduxInputFilter"
            )),
            Some(Event::StarChartOpened)
        );
        assert_eq!(
            classify(&log_line("MapRedux.lua: DBG: HudVis 0")),
            Some(Event::StarChartHidden)
        );
        assert_eq!(classify(&log_line("MapRedux.lua: DBG: HudVis 1")), None);
        assert_eq!(
            classify(&log_line("Created /Lotus/Interface/RadialSolarMapLite.swf")),
            None
        );
    }

    #[test]
    fn empty_squad_mission() {
        assert_eq!(
            classify(&log_line("Set squad mission: ")),
            Some(Event::MissionCleared)
        );
    }

    #[test]
    fn chat_tab_added() {
        let event = classify(&log_line(
            "ChatRedux.lua: ChatRedux::AddTab: Adding tab with channel name: Q_EN_EU to index 1",
        ));
        assert_eq!(
            event,
            Some(Event::ChatTabAdded {
                channel: String::from("Q_EN_EU")
            })
        );
    }

    #[test]
    fn riven_dialog() {
        let event = classify(&log_line(
            "ThemedDetailedPurchaseDialog.lua: PopulateInfo->/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/FuraxWraithRightGauntlet",
        ));
        assert_eq!(
            event,
            Some(Event::RivenDialog {
                item_path:
                    "/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/FuraxWraithRightGauntlet"
                        .to_owned()
            })
        );
    }

    #[test]
    fn reroll_station_loaded() {
        assert_eq!(
            classify(&log_line("OmegaRerollSelection.lua: Diorama setup")),
            Some(Event::RivenRerollScreenLoaded)
        );
    }

    #[test]
    fn scene_teardown_lines() {
        for message in [
            "Flushed 160 bytes of recycled effects.",
            "Flushed 55,680 bytes of recycled effects.",
            "NpcManager::ClearAgents() ReadyToCreateAgents = false",
        ] {
            assert_eq!(
                classify(&log_line(message)),
                Some(Event::SceneTornDown),
                "{message}"
            );
        }
        assert_eq!(
            classify(&log_line(
                "NpcManager::ClearAgents() ReadyToCreateAgents = true"
            )),
            None
        );
    }

    #[test]
    fn input_mapping_reset() {
        for message in [
            "InitMapping for all devices with bindings  and filter ",
            "InitMapping for all devices with bindings /Configs/EE.cfg/LotusWindows_KeyBindings and filter /EE/Types/Input/InputFilter",
            "InitMapping for all devices with bindings /Configs/EE.cfg/LotusWindows_KeyBindings and filter /Lotus/Interface/ProgressInputFilter",
        ] {
            assert_eq!(
                classify(&log_line(message)),
                Some(Event::InputMappingReset),
                "{message}"
            );
        }
    }

    #[test]
    fn console_opened() {
        assert_eq!(
            classify(&log_line(
                "UIConsoleTrigger::Open() /Layer1/Layer2/UIConsoleTrigger4"
            )),
            Some(Event::ConsoleOpened)
        );
        assert_eq!(
            classify(&log_line("      20 /Lotus/Types/Game/UIConsoleTrigger")),
            None
        );
    }

    #[test]
    fn purchase_dialog_hud_vis() {
        assert_eq!(
            classify(&log_line("ThemedDetailedPurchaseDialog.lua: DBG: HudVis 2")),
            Some(Event::PurchaseDialogHudVisibility { count: 2 })
        );
        assert_eq!(
            classify(&log_line("ThemedDetailedPurchaseDialog.lua: DBG: HudVis 1")),
            Some(Event::PurchaseDialogHudVisibility { count: 1 })
        );
        assert_eq!(classify(&log_line("HudRedux.lua: DBG: HudVis 1")), None);
        assert_eq!(
            classify(&log_line("MapRedux.lua: DBG: HudVis 0")),
            Some(Event::StarChartHidden)
        );
    }

    #[test]
    fn inventory_committed() {
        assert_eq!(
            classify(&log_line("CommitInventoryChangesToDB")),
            Some(Event::InventoryCommitted)
        );
    }

    #[test]
    fn inventory_sync_lines() {
        for message in [
            "SyncInventoryFromDB",
            "OnInventoryResults completed in 49ms",
            "Hub.lua: Inventory sync done",
        ] {
            assert_eq!(
                classify(&log_line(message)),
                Some(Event::InventorySynced),
                "{message}"
            );
        }
        assert_eq!(
            classify(&log_line("OnInventoryResults, body size=2059896")),
            None
        );
    }

    #[test]
    fn window_focus() {
        assert_eq!(
            classify(&log_line("WM_ACTIVATEAPP 1")),
            Some(Event::WindowFocus { focused: true })
        );
        assert_eq!(
            classify(&log_line("WM_ACTIVATEAPP 0")),
            Some(Event::WindowFocus { focused: false })
        );
        assert_eq!(classify(&log_line("WM_ACTIVATEAPP")), None);
        assert_eq!(classify(&log_line("WM_ACTIVATEAPP 2")), None);
    }

    #[test]
    fn game_monitor_rectangle() {
        assert_eq!(
            classify(&log_line("Monitor Info (0, 1920, 0, 1080).")),
            Some(Event::GameMonitor(MonitorRect {
                left: 0,
                top: 0,
                width: 1920,
                height: 1080,
            }))
        );
        assert_eq!(
            classify(&log_line("Monitor Info (1920, 4480, -360, 1080).")),
            Some(Event::GameMonitor(MonitorRect {
                left: 1920,
                top: -360,
                width: 2560,
                height: 1440,
            }))
        );
        assert_eq!(classify(&log_line("Monitor Info (0, 0, 0, 1080).")), None);
        assert_eq!(
            classify(&log_line("Monitor Work Area (0, 1920, 32, 1080).")),
            None
        );
    }

    #[test]
    fn unrelated_line() {
        assert_eq!(classify(&log_line("System Up-Time: 6d 13h 54m 20s")), None);
    }

    #[test]
    fn event_label() {
        let logged_in = Event::LoggedIn {
            name: "TestTenno".to_owned(),
        };
        assert_eq!(logged_in.label(), "LoggedIn");
        let received = Event::SquadRewardInfoReceived {
            account_id: "5f0a1b2c3d4e5f6a7b8c9d0e".to_owned(),
        };
        assert_eq!(received.label(), "SquadRewardInfoReceived");
        assert_eq!(Event::InventorySynced.label(), "InventorySynced");
    }
}
