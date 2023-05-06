use std::borrow::{Borrow, BorrowMut};
use std::f32::consts::PI;
use std::process::exit;

use macroquad::prelude::*;
use macroquad::texture::DrawTextureParams;

const MAP_SIZE_X: usize = 100;
const MAP_SIZE_Y: usize = 100;
const MAP_SIZE: usize = MAP_SIZE_X*MAP_SIZE_Y;
const TILE_SIZE: f32 = 10.0;
const ROOT_2: f32 = 1.41421356237;

#[macroquad::main(window_conf)]
async fn main() {
    let mut global = Global::init().await;

    loop {
        global.tick().await;
        next_frame().await;
    }
}

fn window_conf() -> Conf {
    Conf {
        ..Default::default()
    }
}

struct Global {
    state: Option<Game>,
    assets: [Texture2D; 2],
    tick: u32,
}

impl Global {
    async fn init() -> Self {
        Self {
            state: None,
            assets: [
                load_texture("src/assets/play_button.png").await.unwrap(), 
                load_texture("src/assets/logo.png").await.unwrap(),
            ],
            tick: 0,
        }
    }

    async fn tick(&mut self) {
        let mut sw = screen_width();
        let mut sh = screen_height();
        let mouse_position = mouse_position();

        match self.state {
            Some(ref mut game) => {
                clear_background(BLACK);
                if game.tick() {
                    self.state = None;
                    set_camera(&Camera2D::from_display_rect(Rect::new(0.0, 0.0, screen_width(), screen_height())));
                }
            }
            None => {
                if self.tick > 1000000 {
                    let fade = Color::new(1.0-(self.tick-1000000) as f32/100.0, 1.0-(self.tick-1000000) as f32/100.0, 1.0-(self.tick-1000000) as f32/100.0, 1.0);
                    clear_background(fade);

                    draw_texture(self.assets[1], sw/2.0-240.0, sh/2.0-340.0, fade);

                    draw_texture(self.assets[0], sw/2.0-80.0, sh/2.0+80.0+(self.tick as f32/15.0).cos()*15.0, fade);

                    if self.tick > 1000099 {
                        self.state = Some(Game::init().await);
                        self.tick = 0;
                    }
                } else {
                    clear_background(WHITE);
                    draw_texture(self.assets[1], sw/2.0-240.0, sh/2.0-340.0, WHITE);

                    draw_texture(self.assets[0], sw/2.0-80.0, sh/2.0+80.0+(self.tick as f32/15.0).cos()*15.0, WHITE);

                    if is_mouse_button_released(MouseButton::Left) {
                        self.tick = 1000000;
                    }
                }
            }
        }

        self.tick += 1;
    }
}

fn check_hit(line_endpoint1: Vec2, line_endpoint2: Vec2, radius: f32, center: Vec2) -> bool {

    let line_length = ((line_endpoint2.x - line_endpoint1.x).powf(2.0) + (line_endpoint2.y - line_endpoint1.y).powf(2.0)).sqrt();

    let vec_between = Vec2 {
        x: center.x - line_endpoint1.x,
        y: center.y - line_endpoint1.y,
    };

    let projection = ((vec_between.x * (line_endpoint2.x - line_endpoint1.x)) + (vec_between.y * (line_endpoint2.y - line_endpoint1.y))) / line_length;

    if projection < 0.0 || projection > line_length {
        return false;
    }

    let closest_point = Vec2 {
        x: line_endpoint1.x + ((projection / line_length) * (line_endpoint2.x - line_endpoint1.x)),
        y: line_endpoint1.y + ((projection / line_length) * (line_endpoint2.y - line_endpoint1.y)),
    };

    let distance = ((center.x - closest_point.x).powf(2.0) + (center.y - closest_point.y).powf(2.0)).sqrt();

    if distance <= radius {
        return true;
    }

    false
}

struct Game {
    map: [(f32, bool); MAP_SIZE],
    entities: Vec<Entity>,
    player: Player,
    assets: [Texture2D; 10],
}

impl Game {
    async fn init() -> Self {
        let mut init = Self {
            map: [(0.0, false); MAP_SIZE],
            entities: vec![Entity::player(), Entity::player()],
            player: Player::new(),
            assets: [
                load_texture("src/assets/player.png").await.unwrap(), //10x Scale
                load_texture("src/assets/knife.png").await.unwrap(),
                load_texture("src/assets/gunner.png").await.unwrap(),
                load_texture("src/assets/launcher.png").await.unwrap(),
                load_texture("src/assets/shotgun.png").await.unwrap(),
                load_texture("src/assets/sprayer.png").await.unwrap(),
                load_texture("src/assets/sniper.png").await.unwrap(),
                load_texture("src/assets/slot.png").await.unwrap(),
                load_texture("src/assets/grenade.png").await.unwrap(),
                load_texture("src/assets/game_over.png").await.unwrap(),
            ],
        };

        for x in 0..MAP_SIZE_X {
            for y in 0..MAP_SIZE_Y {
                init.map[x*MAP_SIZE_X+y].0 += rand::gen_range(0.0, 0.2);
            }
        }

        init
    }

    fn tick(&mut self) -> bool {
        clear_background(BLACK);
        draw_rectangle_lines(0.0, 0.0, MAP_SIZE_X as f32*50.0, MAP_SIZE_Y as f32*50.0, 20.0, RED);
        
        let mut sw = screen_width();
        let mut sh = screen_height();
        let mut mouse_position = mouse_position();

        for x in 0..MAP_SIZE_X {
            for y in 0..MAP_SIZE_Y {
                draw_rectangle(x as f32*50.0, y as f32*50.0, 50.0, 50.0, Color::new(1.0, 1.0, 1.0, self.map[x*MAP_SIZE_X+y].0))
            }
        }

        let mut appendlist = Vec::new();
        let mut deletelist = Vec::new();
        let mut scope = false;

        if is_key_pressed(KeyCode::Q) {
            appendlist.push(Entity::player())
        }

        let entities = self.entities.to_vec();

        for (count, entity) in self.entities.iter_mut().enumerate() {
            match entity.class {
                Class::Player { ref mut weapon, ref mut direction, ref mut health } => {
                    let recoil = if weapon.last_fire <= 6 {
                        6-weapon.last_fire
                    } else {
                        0
                    };

                    if *health < 100.0 {
                        *health += 0.02;
                    }
                    
                    match weapon.class {
                        WeaponType::Knife(side) => {
                            draw_texture_ex(self.assets[1], entity.position.x+20.0-recoil as f32, entity.position.y-20.0, WHITE, DrawTextureParams {rotation: *direction+if side {1.0} else {-1.0}, pivot: Some(entity.position), flip_y: side, ..Default::default()});
                        }

                        WeaponType::Sniper => {
                            draw_texture_ex(self.assets[6], entity.position.x+20.0-recoil as f32, entity.position.y-20.0, WHITE, DrawTextureParams {rotation: *direction, pivot: Some(entity.position), ..Default::default()});
                        },

                        WeaponType::Gunner => {
                            draw_texture_ex(self.assets[2], entity.position.x+20.0-recoil as f32, entity.position.y-20.0, WHITE, DrawTextureParams {rotation: *direction, pivot: Some(entity.position), ..Default::default()});
                        },

                        WeaponType::Shotgun => {
                            draw_texture_ex(self.assets[4], entity.position.x+20.0-recoil as f32, entity.position.y-20.0, WHITE, DrawTextureParams {rotation: *direction, pivot: Some(entity.position), ..Default::default()});
                        },

                        WeaponType::Sprayer => {
                            draw_texture_ex(self.assets[5], entity.position.x+20.0-recoil as f32, entity.position.y-20.0, WHITE, DrawTextureParams {rotation: *direction, pivot: Some(entity.position), ..Default::default()});
                        },

                        WeaponType::Grenade => {
                            draw_texture_ex(self.assets[3], entity.position.x+20.0-recoil as f32, entity.position.y-20.0, WHITE, DrawTextureParams {rotation: *direction, pivot: Some(entity.position), ..Default::default()});
                        },
                    }

                    //bullet physics
                    let mut preventmultikill = true;
                    for (index, hitbox) in entities.iter().enumerate() {
                        if let Class::Projectile(weapontype, tick, owner) = hitbox.class {
                            let distance = hitbox.position.distance(entity.position);
                            match weapontype {
                                WeaponType::Knife(_side) => {
                                    if count != owner.unwrap() && distance < 60.0 {
                                        *health -= 6.0-tick as f32/60.0;
                                        if !deletelist.contains(&index) {deletelist.push(index);}
                                        entity.velocity.x += hitbox.velocity.x*0.3;
                                        entity.velocity.y += hitbox.velocity.y*0.3;
                                    }
                                }

                                WeaponType::Grenade => {
                                    if tick == 80 {
                                        if distance < 300.0 {
                                            let direction_difference = if hitbox.position.x-entity.position.x > 0.0 {((hitbox.position.y-entity.position.y)/(hitbox.position.x-entity.position.x)).atan()} else {((hitbox.position.y-entity.position.y)/(hitbox.position.x-entity.position.x)).atan()+PI};
                                            *health -= 50.0-distance as f32/6.0;
                                            entity.velocity.x -= direction_difference.cos()*((300.0-distance as f32)/6.0);
                                            entity.velocity.y -= direction_difference.sin()*((300.0-distance as f32)/6.0);
                                        }
                                    }
                                },

                                _ => {
                                    if (check_hit(hitbox.position, Vec2::new(hitbox.position.x+hitbox.velocity.x, hitbox.position.y+hitbox.velocity.y), 60.0, entity.position) || hitbox.position.distance(entity.position) < 60.0) && count != owner.unwrap() {
                                        *health -= match weapontype {
                                            WeaponType::Sniper => 25.0,
                                            WeaponType::Gunner => 8.0,
                                            WeaponType::Shotgun => 3.0,
                                            WeaponType::Sprayer => 7.0,
                                            _ => 0.0,
                                        };

                                        if !deletelist.contains(&index) {deletelist.push(index);} 

                                        appendlist.push(Entity {
                                            position: entity.position,
                                            velocity: Vec2::new(rand::gen_range(0.0, 2.0*PI).cos()*15.0, rand::gen_range(0.0, 2.0*PI).sin()*15.0),
                                            class: Class::Particle(RED, 20)
                                        });
                                        //draw_line(hitbox.position.x, hitbox.position.y, hitbox.velocity.x/3.0, hitbox.velocity.y/3.0, 10.0, WHITE);
                                        entity.velocity.x += hitbox.velocity.x/60.0;
                                        entity.velocity.y += hitbox.velocity.y/60.0;
                                    }
                                },
                            }
                            
                            
                        }
                        
                    }
                    
                    if self.player.index == count {
                        let mouse_diference = Vec2::new(mouse_position.0 - sw/2.0, mouse_position.1 - sh/2.0);
                        *direction = if mouse_diference.x > 0.0 {(mouse_diference.y/mouse_diference.x).atan()} else if mouse_diference.x < 0.0 {PI+(mouse_diference.y/mouse_diference.x).atan()} else {(mouse_diference.y/mouse_diference.x).atan()};
                        
                        if is_quit_requested() {
                            deletelist.push(count);
                        }

                        //weapon change
                        if self.player.game.is_none() {
                            if is_key_down(KeyCode::Key1) {
                                weapon.class = WeaponType::Knife(true);
                            } else if is_key_down(KeyCode::Key2) {
                                weapon.class = WeaponType::Gunner;
                            } else if is_key_down(KeyCode::Key3) {
                                weapon.class = WeaponType::Grenade;
                            } else if is_key_down(KeyCode::Key4) {
                                weapon.class = WeaponType::Shotgun;
                            } else if is_key_down(KeyCode::Key5) {
                                weapon.class = WeaponType::Sprayer;
                            } else if is_key_down(KeyCode::Key6) {
                                weapon.class = WeaponType::Sniper;
                            }
                        

                            //shot detection
                            match weapon.class {
                                WeaponType::Knife(ref mut side) => {
                                    if is_mouse_button_pressed(MouseButton::Left) && weapon.last_fire > 10 {
                                        *side = !*side;
                                        for rotation in -5..5 {
                                            appendlist.push(Entity { 
                                                position: Vec2::new(((rotation as f32)/30.0*PI+*direction).cos()*80.0+entity.position.x, ((rotation as f32)/30.0*PI+*direction).sin()*80.0+entity.position.y),
                                                velocity: Vec2::new(((rotation as f32)/30.0*PI+*direction).cos()*2.0+entity.velocity.x, ((rotation as f32)/30.0*PI+*direction).sin()*2.0+entity.velocity.y),
                                                class: Class::Projectile(WeaponType::Knife(*side), 0, Some(self.player.index)),
                                            })
                                        }
                                        weapon.last_fire = 0;
                                    }
                                    
                                },

                                WeaponType::Sniper => {
                                    scope = true;
                                    if is_mouse_button_pressed(MouseButton::Left) && weapon.last_fire > 30 {
                                        appendlist.push(Entity { 
                                            position: Vec2::new(entity.position.x+direction.cos()*90.0, entity.position.y+direction.sin()*90.0),
                                            velocity: Vec2::new(entity.velocity.x+direction.cos()*150.0, entity.velocity.y+direction.sin()*150.0),
                                            class: Class::Projectile(WeaponType::Sniper, 0, Some(self.player.index)),
                                        });

                                        for rotation in -5..6 {
                                            appendlist.push(Entity { 
                                                position: Vec2::new(direction.cos()*100.0+entity.position.x+((rotation as f32)/10.0*PI+*direction).cos()*10.0, direction.sin()*100.0+entity.position.y+((rotation as f32)/10.0*PI+*direction).cos()*10.0),
                                                velocity: Vec2::new(((rotation as f32)/10.0*PI+*direction).cos()*2.0+entity.velocity.x, ((rotation as f32)/10.0*PI+*direction).sin()*2.0+entity.velocity.y),
                                                class: Class::Particle(ORANGE, 15),
                                            })
                                        }
                                        weapon.last_fire = 0;
                                        
                                        entity.position.x -= direction.cos()*10.0;
                                        entity.position.y -= direction.sin()*10.0;
                                    }
                                },

                                WeaponType::Gunner => {
                                    if is_mouse_button_down(MouseButton::Left) && weapon.last_fire > 10 {
                                        appendlist.push(Entity { 
                                            position: Vec2::new(entity.position.x+direction.cos()*90.0, entity.position.y+direction.sin()*90.0),
                                            velocity: Vec2::new(entity.velocity.x+direction.cos()*120.0, entity.velocity.y+direction.sin()*120.0),
                                            class: Class::Projectile(WeaponType::Gunner, 0, Some(self.player.index)),
                                        });

                                        for rotation in -2..3 {
                                            appendlist.push(Entity { 
                                                position: Vec2::new(direction.cos()*100.0+entity.position.x+((rotation as f32)/10.0*PI+*direction).cos()*10.0, direction.sin()*100.0+entity.position.y+((rotation as f32)/10.0*PI+*direction).cos()*10.0),
                                                velocity: Vec2::new(((rotation as f32)/10.0*PI+*direction).cos()*2.0+entity.velocity.x, ((rotation as f32)/10.0*PI+*direction).sin()*2.0+entity.velocity.y),
                                                class: Class::Particle(ORANGE, 15),
                                            })
                                        }
                                        weapon.last_fire = 0;
                                        entity.velocity.x -= direction.cos()*5.0;
                                        entity.velocity.y -= direction.sin()*5.0;
                                    }
                                    
                                },

                                WeaponType::Shotgun => {
                                    if is_mouse_button_pressed(MouseButton::Left) && weapon.last_fire > 30 {
                                        for rotation in -5..6 {
                                            appendlist.push(Entity { 
                                                position: Vec2::new(entity.position.x+direction.cos()*90.0, entity.position.y+direction.sin()*90.0),
                                                velocity: Vec2::new(entity.velocity.x+((rotation as f32)/30.0*PI+*direction).cos()*70.0, entity.velocity.y+((rotation as f32)/30.0*PI+*direction).sin()*70.0),
                                                class: Class::Projectile(WeaponType::Shotgun, 0, Some(self.player.index)),
                                            });
                                        }

                                        for rotation in -2..3 {
                                            appendlist.push(Entity { 
                                                position: Vec2::new(direction.cos()*100.0+entity.position.x+((rotation as f32)/10.0*PI+*direction).cos()*10.0, direction.sin()*100.0+entity.position.y+((rotation as f32)/10.0*PI+*direction).cos()*10.0),
                                                velocity: Vec2::new(((rotation as f32)/10.0*PI+*direction).cos()*2.0+entity.velocity.x, ((rotation as f32)/10.0*PI+*direction).sin()*2.0+entity.velocity.y),
                                                class: Class::Particle(ORANGE, 15),
                                            })
                                        }
                                        weapon.last_fire = 0;

                                        entity.velocity.x -= direction.cos()*12.0;
                                        entity.velocity.y -= direction.sin()*12.0;
                                    }
                                    
                                },

                                WeaponType::Sprayer => {
                                    if is_mouse_button_down(MouseButton::Left) && weapon.last_fire > 3 {
                                        let shotdirection = *direction+PI*rand::gen_range(-0.1, 0.1);
                                        appendlist.push(Entity { 
                                            position: Vec2::new(entity.position.x+direction.cos()*100.0, entity.position.y+direction.sin()*100.0),
                                            velocity: Vec2::new(entity.velocity.x+shotdirection.cos()*90.0, entity.velocity.y+shotdirection.sin()*90.0),
                                            class: Class::Projectile(WeaponType::Shotgun, 0, Some(self.player.index)),
                                        });


                                        for rotation in -2..3 {
                                            appendlist.push(Entity { 
                                                position: Vec2::new(direction.cos()*110.0+entity.position.x+((rotation as f32)/10.0*PI+*direction).cos()*10.0, direction.sin()*110.0+entity.position.y+((rotation as f32)/10.0*PI+*direction).cos()*10.0),
                                                velocity: Vec2::new(((rotation as f32)/10.0*PI+*direction).cos()*2.0+entity.velocity.x, ((rotation as f32)/10.0*PI+*direction).sin()*2.0+entity.velocity.y),
                                                class: Class::Particle(ORANGE, 15),
                                            })
                                        }
                                        weapon.last_fire = 0;
                                        entity.velocity.x -= direction.cos()*2.0;
                                        entity.velocity.y -= direction.sin()*2.0;
                                    }

                                    
                                }

                                WeaponType::Grenade => {
                                    if is_mouse_button_pressed(MouseButton::Left) && weapon.last_fire > 70 {
                                        appendlist.push(Entity { 
                                            position: Vec2::new(entity.position.x+direction.cos()*90.0, entity.position.y+direction.sin()*90.0),
                                            velocity: Vec2::new(entity.velocity.x+direction.cos()*40.0, entity.velocity.y+direction.sin()*40.0),
                                            class: Class::Projectile(WeaponType::Grenade, 0, Some(self.player.index)),
                                        });

                                        for rotation in -2..3 {
                                            appendlist.push(Entity { 
                                                position: Vec2::new(direction.cos()*110.0+entity.position.x+((rotation as f32)/10.0*PI+*direction).sin()*10.0, direction.sin()*110.0+entity.position.y+((rotation as f32)/10.0*PI+*direction).sin()*10.0),
                                                velocity: Vec2::new(((rotation as f32)/10.0*PI+*direction).cos()*5.0+entity.velocity.x, ((rotation as f32)/10.0*PI+*direction).sin()*5.0+entity.velocity.y),
                                                class: Class::Particle(GREEN, 15),
                                            })
                                        }
                                        weapon.last_fire = 0;
                                        
                                        entity.velocity.x -= direction.cos()*10.0;
                                        entity.velocity.y -= direction.sin()*10.0;
                                    }
                                    
                                },

                                _ => {

                                },
                            }

                            let (mut x_change, mut y_change): (f32, f32) = (0.0, 0.0);
                            if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
                                y_change -= 1.0;
                            } if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
                                y_change += 1.0;
                            } if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
                                x_change -= 1.0;
                            } if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
                                x_change += 1.0;
                            }

                            if x_change.abs() + y_change.abs() == 2.0 {
                                x_change /= ROOT_2;
                                y_change /= ROOT_2;
                            }

                            entity.velocity.x += x_change;
                            entity.velocity.y += y_change;
                        }

                        weapon.last_fire += 1;

                        draw_texture_ex(self.assets[0], entity.position.x-30.0, entity.position.y-30.0,  BLUE, DrawTextureParams { rotation: *direction, ..Default::default() });

                        if *health < 0.0 && preventmultikill && self.player.game.is_none() {
                            deletelist.push(count);
                            preventmultikill = false; 
                            self.player.game = Some(0);                 
                                
                            for rotation in 0..30 {
                                let rotation = rotation as f32/15.0*PI;
                                appendlist.push(Entity {
                                    position: entity.position,
                                    velocity: Vec2::new(rotation.cos()*15.0, rotation.sin()*15.0),
                                    class: Class::Particle(RED, 20)
                                });
    
                                appendlist.push(Entity {
                                    position: entity.position,
                                    velocity: Vec2::new(rotation.cos()*10.0, rotation.sin()*10.0),
                                    class: Class::Gold(rotation),
                                });
                                        
                            }
                        }
                        
                    } else {
                        draw_texture_ex(self.assets[0], entity.position.x-30.0, entity.position.y-30.0,  RED, DrawTextureParams { rotation: *direction, ..Default::default() });
                        let text_center = get_text_center(&format!("{:?}", *health as i8), None, 30, 1.0, 0.0);
                        draw_text(&format!("{:?}", *health as i8), entity.position.x-text_center.x, entity.position.y+50.0-text_center.y, 30.0, RED);
                    
                        if *health < 0.0 && preventmultikill {
                            deletelist.push(count);
                            preventmultikill = false;                  
                                
                            for rotation in 0..30 {
                                let rotation = rotation as f32/15.0*PI;
                                appendlist.push(Entity {
                                    position: entity.position,
                                    velocity: Vec2::new(rotation.cos()*15.0, rotation.sin()*15.0),
                                    class: Class::Particle(RED, 20)
                                });
    
                                appendlist.push(Entity {
                                    position: entity.position,
                                    velocity: Vec2::new(rotation.cos()*10.0, rotation.sin()*10.0),
                                    class: Class::Gold(rotation),
                                });
                                        
                            }
                        }
                    }

                    for x in 0..MAP_SIZE_X {
                        for y in 0..MAP_SIZE_Y {
                            if self.map[x*MAP_SIZE_X+y].1 {
                                let hitbox = &Rect::new(x as f32*50.0, y as f32*50.0, 50.0, 50.0);
                                if check_box_hit(entity.position, entity.position+entity.velocity, hitbox)  {
                                    let direction = if entity.position.x-hitbox.x > 0.0 {((entity.position.y-hitbox.y)/(entity.position.x-hitbox.x)).atan()} else {((entity.position.y-hitbox.y)/(entity.position.x-hitbox.x)).atan()+PI};
                                    entity.velocity *= if direction < PI*-0.25 {
                                        Vec2::new(0.0, 1.0)
                                    } else if direction < PI*0.25 {
                                        Vec2::new(1.0, 0.0)
                                    } else if direction < PI*0.75 {
                                        Vec2::new(0.0, -1.0)
                                    } else if direction < PI*1.25 {
                                        Vec2::new(1.0, 0.0)
                                    } else {
                                        Vec2::new(0.0, 1.0)
                                    };

                                    entity.position.x += entity.velocity.x;
                                    entity.position.y += entity.velocity.y;
                                }
                            }
                        }
                    }

                    if entity.position.x > MAP_SIZE_X as f32*50.0 || entity.position.y > MAP_SIZE_Y as f32*50.0 || entity.position.x < 0.0 || entity.position.y < 0.0 {
                        *health -= 1.0;
                    }
                },       

                Class::Gold(ref mut tick) => {
                    draw_rectangle(entity.position.x, entity.position.y, 10.0, 10.0, Color::new(1.0, 0.84+tick.sin()*0.1, 0.0, 1.0));
                    *tick += 0.1;

                    for hitbox in entities.iter() {
                        if let Class::Player { weapon: _, direction: _, health: _ } = hitbox.class {
                            let distance = hitbox.position.distance(entity.position);
                            if distance < 100.0 {
                                let direction = if entity.position.x-hitbox.position.x > 0.0 {
                                    ((entity.position.y - hitbox.position.y)/(entity.position.x - hitbox.position.x)).atan()+PI
                                } else {
                                    ((entity.position.y - hitbox.position.y)/(entity.position.x - hitbox.position.x)).atan()
                                };
                                entity.velocity.x += direction.cos();
                                entity.velocity.y += direction.sin();

                                if distance < 40.0 {
                                    self.player.gold += 1;
                                    deletelist.push(count);
                                }
                            }
                        }
                    }
                },

                Class::Particle(ref mut color, ref mut fade) => {
                    draw_rectangle(entity.position.x, entity.position.y, 10.0, 10.0, *color);
                    *fade -= 1;

                    if *fade < 30 {
                        (*color).a = *fade as f32/30.0;
                        if *fade == 0 {
                            deletelist.push(count);
                        }
                    }
                },

                Class::Projectile(weapontype, ref mut tick, owner) => {
                    match weapontype {
                        WeaponType::Knife(_) => {                            
                            draw_rectangle(entity.position.x-5.0, entity.position.y-5.0, 10.0, 10.0, Color::new(1.0-(*tick as f32)/20.0, 1.0-(*tick as f32)/20.0, 1.0-(*tick as f32)/20.0, 1.0));
                            if *tick >= 15 {
                                deletelist.push(count);
                            }
                        },

                        WeaponType::Grenade => {
                            draw_texture_ex(self.assets[8], entity.position.x-20.0, entity.position.y-20.0, WHITE, DrawTextureParams {rotation: *tick as f32/20.0, ..Default::default()});
                            if *tick >= 80 {
                                deletelist.push(count);
                            for rotation in 0..30 {
                                let rotation = rotation as f32/15.0*PI;
                                appendlist.push(Entity {
                                    position: entity.position,
                                    velocity: Vec2::new(rotation.cos()*15.0, rotation.sin()*15.0),
                                    class: Class::Particle(RED, 25)
                                });

                                appendlist.push(Entity {
                                    position: entity.position,
                                    velocity: Vec2::new((rotation+0.1).cos()*17.0, (rotation+0.1).sin()*17.0),
                                    class: Class::Particle(RED, 20)
                                });
                            }
                            }

                            
                        },

                        _ => {
                            if !deletelist.contains(&count) {
                                draw_line(entity.position.x, entity.position.y, entity.position.x+entity.velocity.x, entity.position.y+entity.velocity.y, 10.0, WHITE);
                                if entity.velocity.distance(Vec2::new(0.0, 0.0)) < 5.0 {
                                    deletelist.push(count)
                                }
                            }
                            
                            for i in 0..entity.velocity.length() as u32 {
                                /*for hitbox in 0..entities.len() {
                                    if entities[hitbox].position.distance(Vec2::new(entity.position.x + (i as f32).cos(), entity.position.y + (i as f32).cos())) < 40.0 {
                                        //deal damage
                                    }
                                }*/
                            }
                        },

                    };

                    *tick += 1
                },
            }

            

            entity.position.x += entity.velocity.x;
            entity.position.y += entity.velocity.y;

            entity.velocity.x *= 0.90;
            entity.velocity.y *= 0.90;
        }

        deletelist.sort();
        for (count, index) in deletelist.iter().enumerate() {
            self.entities.remove(index-count);
            if index-count < self.player.index {
                self.player.index -= 1;
            }
        }

        

        self.entities.append(&mut appendlist);

        mouse_position = ((mouse_position_local().x+1.0)/2.0, (mouse_position_local().y+1.0)/2.0);
        if sh > sw {
            sw = sw/sh * 1600.0;
            sh = 1600.0;
            mouse_position.0 *= sw;
            mouse_position.1 *= sh;
        } else {
            sh = sh/sw * 1600.0;
            sw = 1600.0;
            mouse_position.0 *= sw;
            mouse_position.1 *= sh;
        }

        if scope {
            sw *= 1.1;
            sh *= 1.1;
        }        

        self.player.camera = Camera2D::from_display_rect(Rect { x: self.entities[self.player.index].position.x - sw/2.0, y: self.entities[self.player.index].position.y - sh/2.0, w: sw, h: sh, });
        set_camera(&self.player.camera);
        

        for index in 1..7 {
            draw_texture_ex(self.assets[index],  self.entities[self.player.index].position.x-370.0+(index as f32*100.0), self.entities[self.player.index].position.y+sh/2.0-65.0, WHITE, DrawTextureParams {rotation: -PI/4.0, dest_size: Some(Vec2::new(67.5, 30.0)),  ..Default::default()});
            let selected =  if let Class::Player { ref mut weapon, direction: _, health: _ } = self.entities[self.player.index].class {
                if is_mouse_button_pressed(MouseButton::Left) {
                    if mouse_position.0 > sw/2.0-375.0+(index as f32*100.0) && mouse_position.0 < sw/2.0-295.0+(index as f32*100.0) && mouse_position.1 > sh-90.0 {
                        (*weapon).class = match index {
                            1 => WeaponType::Knife(true),
                            2 => WeaponType::Gunner,
                            3 => WeaponType::Grenade,
                            4 => WeaponType::Shotgun,
                            5 => WeaponType::Sprayer,
                            6 => WeaponType::Sniper,
                            _ => WeaponType::Knife(false),
                        };
                    }
                };

                if match weapon.class { 
                    WeaponType::Gunner => 2, 
                    WeaponType::Sniper => 6,
                    WeaponType::Shotgun => 4,
                    WeaponType::Sprayer => 5,
                    WeaponType::Grenade => 3,
                    WeaponType::Knife(_) => 1, 
                } == index {true} else {false}
            } else {false};
            draw_texture_ex(self.assets[7], self.entities[self.player.index].position.x-375.0+(index as f32*100.0), self.entities[self.player.index].position.y+sh/2.0-90.0, if selected {BLUE} else {WHITE}, DrawTextureParams {..Default::default()});
        }

        draw_text(&format!("{:?}", self.player.gold), 10.0+self.entities[self.player.index].position.x-sw/2.0, 120.0+self.entities[self.player.index].position.y-sh/2.0, 80.0, YELLOW);            
        draw_text(&format!("{:?}", if let Class::Player { weapon: _, direction: _, health } = self.entities[self.player.index].class {health as u8} else {0.0 as u8}), 10.0+self.entities[self.player.index].position.x-sw/2.0, 60.0+self.entities[self.player.index].position.y-sh/2.0, 80.0, RED);            

        if self.player.game.is_some() {
            draw_rectangle(self.entities[self.player.index].position.x-sw/2.0, self.entities[self.player.index].position.y-sh/2.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_texture(self.assets[9], self.entities[self.player.index].position.x-320.0, self.entities[self.player.index].position.y-160.0+(self.player.game.unwrap() as f32/20.0).cos()*10.0, WHITE);

            if self.player.game.unwrap() > 400 {
                return true;
            }

            self.player.game = Some(self.player.game.unwrap()+1);
        }

        false
    }
}

fn check_box_hit(line_start: Vec2, line_end: Vec2, rect: &Rect) -> bool {
    let left = rect.x;
    let right = rect.x + rect.w;
    let top = rect.y;
    let bottom = rect.y + rect.h;

    let left_intersect = line_intersects_line(line_start, line_end, Vec2::new(left, top), Vec2::new(left, bottom));
    let right_intersect = line_intersects_line(line_start, line_end, Vec2::new(right, top), Vec2::new(right, bottom));
    let top_intersect = line_intersects_line(line_start, line_end, Vec2::new(left, top), Vec2::new(right, top));
    let bottom_intersect = line_intersects_line(line_start, line_end, Vec2::new(left, bottom), Vec2::new(right, bottom));

    left_intersect || right_intersect || top_intersect || bottom_intersect
}

fn line_intersects_line(a_start: Vec2, a_end: Vec2, b_start: Vec2, b_end: Vec2) -> bool {
    let a_slope = (a_end.y - a_start.y) / (a_end.x - a_start.x);
    let a_intercept = a_start.y - (a_slope * a_start.x);
    let b_slope = (b_end.y - b_start.y) / (b_end.x - b_start.x);
    let b_intercept = b_start.y - (b_slope * b_start.x);

    if a_slope == b_slope {
        return false;
    }

    let x = (b_intercept - a_intercept) / (a_slope - b_slope);
    let y = (a_slope * x) + a_intercept;

    let a_within_bounds = x >= a_start.x.min(a_end.x) && x <= a_start.x.max(a_end.x) && y >= a_start.y.min(a_end.y) && y <= a_start.y.max(a_end.y);
    let b_within_bounds = x >= b_start.x.min(b_end.x) && x <= b_start.x.max(b_end.x) && y >= b_start.y.min(b_end.y) && y <= b_start.y.max(b_end.y);

    a_within_bounds && b_within_bounds
}

struct Player {
    gold: u32,
    index: usize,
    camera: Camera2D,
    game: Option<u16>,
}

impl Player {
    fn new() -> Self {
        Self {
            gold: 0,
            index: 0,
            camera: Camera2D::from_display_rect(Rect::new(0.0, 0.0, 1.0, 1.0,)),
            game: None,
        }
    }
}

#[derive(Clone, Copy)]
struct Entity {
    position: Vec2,
    velocity: Vec2,
    class: Class, 
}

impl Entity {
    fn player() -> Self {
        Self {
            position: Vec2::new(rand::gen_range(100.0, MAP_SIZE_X as f32*50.0-100.0), rand::gen_range(100.0, MAP_SIZE_Y as f32*50.0-100.0)),
            velocity: Vec2::new(0.0, 0.0),
            class: Class::Player { weapon: Weapon { class: {let gen = rand::gen_range(0.0, 6.0); if gen < 1.0 {WeaponType::Sniper} else if gen < 2.0 {WeaponType::Gunner} else if gen < 3.0 {WeaponType::Grenade} else if gen < 4.0 {WeaponType::Shotgun} else if gen < 5.0 {WeaponType::Sprayer} else {WeaponType::Knife(rand::gen_range(0.0, 2.0) < 1.0)}}, last_fire: 0 }, direction: rand::gen_range(-PI, PI), health: 100.0 },
        }
    }
}

#[derive(Clone, Copy)]
enum Class {
    Player {
        weapon: Weapon,
        direction: f32,
        health: f32,
    },

    Gold(f32),

    Particle(Color, u16),

    Projectile(WeaponType, u16, Option<usize>),
}

#[derive(Clone, Copy)]
struct Weapon {
    class: WeaponType,
    last_fire: u32,
}

#[derive(Clone, Copy)]
enum WeaponType {
    Sniper, //
    Gunner, //
    Shotgun, 
    Sprayer,
    Grenade, //
    Knife(bool), //
}