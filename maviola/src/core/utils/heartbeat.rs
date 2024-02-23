use crate::prelude::*;

pub(crate) fn make_heartbeat_message<D: Dialect>() -> mavio::dialects::minimal::messages::Heartbeat
{
    use crate::dialects::minimal as dialect;

    dialect::messages::Heartbeat {
        type_: Default::default(),
        autopilot: dialect::enums::MavAutopilot::Generic,
        base_mode: Default::default(),
        custom_mode: 0,
        system_status: dialect::enums::MavState::Active,
        mavlink_version: D::version().unwrap_or_default(),
    }
}
