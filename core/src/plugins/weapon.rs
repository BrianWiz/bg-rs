use bevy::prelude::*;

use crate::components::{AimYaw, Velocity, Weapon, WeaponState, WeaponWishFire};

use super::shared::Ticks;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, update_weapon_system);
    }
}

fn update_weapon_system(
    ticks: Res<Ticks>,
    mut weapons_wanting_to_fire: Query<(
        &mut Velocity,
        &AimYaw,
        &Weapon,
        &mut WeaponState,
        &WeaponWishFire,
    )>,
) {
    for (mut velocity, aim_yaw, weapon, mut state, wish_fire) in weapons_wanting_to_fire.iter_mut()
    {
        let wishes_to_fire = wish_fire.0;
        if state.next_fire_tick <= ticks.ticks && wishes_to_fire {
            state.next_fire_tick = ticks.ticks + weapon.fire_rate_ticks;

            if let Some(recoil) = weapon.recoil {
                // by multiplying it with a vec3, we are converting it to a direction
                let recoil_direction = Quat::from_rotation_y(aim_yaw.0) * Vec3::NEG_Z;
                velocity.0 += recoil_direction * recoil;
            }

            info!("Firing weapon");
        }
    }
}
