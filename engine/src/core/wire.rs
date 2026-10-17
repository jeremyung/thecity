//! Binary layouts the worker and renderer decode. This is the single definition:
//! `export_bindings_wire_layout` writes it to web/src/generated/wire.ts.

/// Floats per actor in the movement buffer: id, x, z, heading, kind, surface y.
pub const MOVEMENT_STRIDE: usize = 6;
pub const MOVEMENT_ID: usize = 0;
pub const MOVEMENT_X: usize = 1;
pub const MOVEMENT_Z: usize = 2;
pub const MOVEMENT_HEADING: usize = 3;
pub const MOVEMENT_KIND: usize = 4;
pub const MOVEMENT_SURFACE_Y: usize = 5;

/// Actor kinds carried in the movement buffer's kind field.
pub const ACTOR_PERSON: u8 = 0;
pub const ACTOR_COMPACT: u8 = 1;
pub const ACTOR_TRAIN: u8 = 2;
pub const ACTOR_BUS: u8 = 3;
pub const ACTOR_KEI: u8 = 4;
pub const ACTOR_TAXI: u8 = 5;
pub const ACTOR_VAN: u8 = 6;
pub const ACTOR_TRUCK: u8 = 7;
pub const ACTOR_MAIL: u8 = 8;

/// Floats per train in the door buffer: id, terminal node (-1 away), opening 0..1.
pub const TRAIN_DOOR_STRIDE: usize = 3;

/// Version word that leads every movement support packet.
pub const MOVEMENT_PACKET_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_bindings_wire_layout() {
        // `npm run types` sets the directory; plain test runs write nothing.
        let Ok(dir) = std::env::var("TS_RS_EXPORT_DIR") else {
            return;
        };
        let text = format!(
            "// Generated from engine/src/core/wire.rs. Do not edit.\n\
             /** Floats per actor in the movement buffer: id, x, z, heading, kind, surface y. */\n\
             export const MOVEMENT_STRIDE = {MOVEMENT_STRIDE};\n\
             export const MOVEMENT_FIELD = {{ id: {MOVEMENT_ID}, x: {MOVEMENT_X}, z: {MOVEMENT_Z}, heading: {MOVEMENT_HEADING}, kind: {MOVEMENT_KIND}, surfaceY: {MOVEMENT_SURFACE_Y} }} as const;\n\
             /** Actor kinds carried in the movement buffer's kind field. */\n\
             export const MOVEMENT_KINDS = {{ person: {ACTOR_PERSON}, compact: {ACTOR_COMPACT}, train: {ACTOR_TRAIN}, bus: {ACTOR_BUS}, kei: {ACTOR_KEI}, taxi: {ACTOR_TAXI}, van: {ACTOR_VAN}, truck: {ACTOR_TRUCK}, mail: {ACTOR_MAIL} }} as const;\n\
             /** Floats per train in the door buffer: id, terminal node (-1 away), opening 0..1. */\n\
             export const TRAIN_DOOR_STRIDE = {TRAIN_DOOR_STRIDE};\n\
             /** Version word that leads every movement support packet. */\n\
             export const MOVEMENT_PACKET_VERSION = {MOVEMENT_PACKET_VERSION};\n"
        );
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/wire.ts"), text).unwrap();
    }
}
