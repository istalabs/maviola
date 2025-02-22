use crate::protocol::DialectVersion;

pub(crate) fn make_heartbeat_message(
    version: Option<DialectVersion>,
) -> mavio::dialects::minimal::messages::Heartbeat {
    use crate::protocol::dialects::minimal as dialect;

    dialect::messages::Heartbeat {
        type_: Default::default(),
        autopilot: dialect::enums::MavAutopilot::Generic,
        base_mode: Default::default(),
        custom_mode: 0,
        system_status: dialect::enums::MavState::Active,
        mavlink_version: version.unwrap_or_default(),
    }
}
