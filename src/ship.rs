use bevy::prelude::*;
// use crate::factions::Faction;
// use serde::{Deserialize, Serialize};

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_player_ship)
            .add_systems(Update, (
                update_ship_systems,
                handle_ship_damage,
                update_power_distribution,
            ));
    }
}

#[derive(Component)]
pub struct PlayerShip;

#[derive(Component)]
pub struct Ship {
    pub systems: ShipSystems,
    pub weapons: Vec<Weapon>,
}


#[derive(Component, Clone)]
pub struct ShipSystems {
    pub engines: SystemModule,
    pub weapons: SystemModule,
    pub shields: SystemModule,
    pub oxygen: SystemModule,
    pub medbay: SystemModule,
    pub sensors: SystemModule,
}

#[derive(Clone)]
pub struct SystemModule {
    pub level: u32,
    pub power_allocated: u32,
    pub health: f32,
    pub efficiency: f32, // 0.0 to 1.0
}

impl SystemModule {
    pub fn new() -> Self {
        Self {
            level: 1,
            power_allocated: 1,
            health: 100.0,
            efficiency: 1.0,
        }
    }

    pub fn is_functional(&self) -> bool {
        self.health > 0.0 && self.power_allocated > 0
    }

    pub fn effective_level(&self) -> f32 {
        if !self.is_functional() {
            return 0.0;
        }
        (self.power_allocated as f32).min(self.level as f32) * self.efficiency
    }
}

#[derive(Component, Clone)]
pub struct Weapon {
    pub charge_time: f32,
    pub current_charge: f32,
}

#[derive(Component)]
pub struct Shields {
    pub current: f32,
    pub max: f32,
    pub recharge_rate: f32,
    pub recharge_delay: f32,
    pub last_hit_time: f32,
}

#[derive(Resource)]
pub struct PowerDistribution {
    pub total_power: u32,
    pub available_power: u32,
}

fn setup_player_ship(mut commands: Commands) {
    // Create the player's starting ship based on Cosmicrafts lore
    let ship = Ship {
        systems: ShipSystems {
            engines: SystemModule::new(),
            weapons: SystemModule::new(),
            shields: SystemModule::new(),
            oxygen: SystemModule::new(),
            medbay: SystemModule::new(),
            sensors: SystemModule::new(),
        },
        weapons: vec![
            Weapon {
                charge_time: 2.0,
                current_charge: 0.0,
            }
        ],
    };

    let shields = Shields {
        current: 2.0,
        max: 2.0,
        recharge_rate: 1.0,
        recharge_delay: 5.0,
        last_hit_time: 0.0,
    };

    // Ship data without visual representation (map handles visuals)
    commands.spawn((
        PlayerShip,
        ship,
        shields,
    ));

    // Initialize power distribution
    commands.insert_resource(PowerDistribution {
        total_power: 8,
        available_power: 8,
    });
}

fn update_ship_systems(
    mut ships: Query<(&mut Ship, &mut Shields)>,
    time: Res<Time>,
) {
    for (mut ship, mut shields) in ships.iter_mut() {
        // Update weapon charging
        for weapon in &mut ship.weapons {
            if weapon.current_charge < weapon.charge_time {
                weapon.current_charge += time.delta_secs();
            }
        }

        // Update shield recharge
        let current_time = time.elapsed_secs();
        if current_time - shields.last_hit_time > shields.recharge_delay {
            if shields.current < shields.max {
                let shield_power = ship.systems.shields.effective_level();
                shields.current = (shields.current + shields.recharge_rate * shield_power * time.delta_secs())
                    .min(shields.max);
            }
        }

        // Update system efficiency based on damage
        update_system_efficiency(&mut ship.systems.engines);
        update_system_efficiency(&mut ship.systems.weapons);
        update_system_efficiency(&mut ship.systems.shields);
        update_system_efficiency(&mut ship.systems.oxygen);
        update_system_efficiency(&mut ship.systems.medbay);
        update_system_efficiency(&mut ship.systems.sensors);
    }
}

fn update_system_efficiency(system: &mut SystemModule) {
    // Systems become less efficient as they take damage
    system.efficiency = (system.health / 100.0).max(0.25);
}

fn handle_ship_damage(
    // This system will handle incoming damage to ships
    // For now, it's a placeholder for the combat system
) {
    // Placeholder for damage handling
}

fn update_power_distribution(
    mut power_dist: ResMut<PowerDistribution>,
    mut ships: Query<&mut Ship, With<PlayerShip>>,
) {
    if let Ok(ship) = ships.single_mut() {
        // Calculate total power usage
        let total_used = ship.systems.engines.power_allocated
            + ship.systems.weapons.power_allocated
            + ship.systems.shields.power_allocated
            + ship.systems.oxygen.power_allocated
            + ship.systems.medbay.power_allocated
            + ship.systems.sensors.power_allocated;

        power_dist.available_power = power_dist.total_power.saturating_sub(total_used);
    }
}

// Combat-related functions
