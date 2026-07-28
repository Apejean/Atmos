use crate::core::state::GLOBAL_STATE;
use crate::common::commands::AudioCommand;
use crate::common::config::Point3D;
use std::time::Instant;

pub fn start_scheduler() {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(33)); // ~30Hz
            let mut last_tick = Instant::now();
            
            loop {
                interval.tick().await;
                if !crate::api::simple::ENGINE_ACTIVE.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                
                let now = Instant::now();
                let dt = now.duration_since(last_tick).as_secs_f32();
                last_tick = now;
                
                // Read global trajectory
                let (has_trajectory, current_pos, target_pos, speed) = {
                    let config_guard = GLOBAL_STATE.config.read().unwrap();
                    if let Some(config) = config_guard.as_ref() {
                        if let Some(traj) = &config.global_trajectory {
                            if !traj.waypoints.is_empty() {
                                // For simplicity, we just move towards the first waypoint
                                // In a real system, we would track index and interpolate
                                (true, traj.current_position.clone(), traj.waypoints[0].clone(), 2.0_f32) // Default speed 2m/s
                            } else {
                                (false, Point3D { x: 0.0, y: 0.0, z: 0.0 }, Point3D { x: 0.0, y: 0.0, z: 0.0 }, 0.0)
                            }
                        } else {
                            (false, Point3D { x: 0.0, y: 0.0, z: 0.0 }, Point3D { x: 0.0, y: 0.0, z: 0.0 }, 0.0)
                        }
                    } else {
                        (false, Point3D { x: 0.0, y: 0.0, z: 0.0 }, Point3D { x: 0.0, y: 0.0, z: 0.0 }, 0.0)
                    }
                };
                
                if has_trajectory {
                    let dx = target_pos.x - current_pos.x;
                    let dy = target_pos.y - current_pos.y;
                    let dz = target_pos.z - current_pos.z;
                    let dist = (dx*dx + dy*dy + dz*dz).sqrt();
                    
                    if dist > 0.01 {
                        let move_dist = (speed * dt).min(dist);
                        let ratio = move_dist / dist;
                        let new_pos = Point3D {
                            x: current_pos.x + dx * ratio,
                            y: current_pos.y + dy * ratio,
                            z: current_pos.z + dz * ratio,
                        };
                        
                        // Update config state
                        {
                            let mut config_guard = GLOBAL_STATE.config.write().unwrap();
                            if let Some(config) = config_guard.as_mut() {
                                if let Some(traj) = &mut config.global_trajectory {
                                    traj.current_position = new_pos.clone();
                                }
                            }
                        }
                        
                        // Send update to audio thread
                        let _ = GLOBAL_STATE.command_sender.lock().unwrap().push(AudioCommand::UpdateTrajectoryPosition {
                            position: new_pos,
                        });
                    }
                }
            }
        });
    });
}
