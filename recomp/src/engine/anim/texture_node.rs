//! EFTextureAnimNode (Classes/EF/EFTextureAnimNode.cpp, class 0x54 "EF_TEXT_ANIM_NODE").
//! Replaced: RpMaterialSetTexture / RwTexDictionaryFindNamedTexture / the clump-atomic and
//! world-material searches of Connect. The node exposes the texture NAME it would set
//! ([`EFTextureAnimNode::current_texture`]) and the renderer (WP-C) applies it to the material(s)
//! the connect step found. The EF3dObj texture-anim link list (obj+0x5c/+0x60/+0x64) is not kept.
#![allow(non_snake_case)]

use super::nodes::{AnimEvent, state};
use super::res::EFTextureAnimRes;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct EFTextureAnimNode {
    /// this+0x20 notification user value
    pub user: u32,
    /// this+0x24 (Initialize sets 0)
    pub excl: u32,
    /// this+0x2c connected EFInstance (opaque id)
    pub obj: u32,
    /// this+0x30
    pub res: Arc<EFTextureAnimRes>,
    /// this+0x34
    pub state: u32,
    /// this+0x38
    pub time: f32,
    /// this+0x3c = time of the last frame
    pub duration: f32,
    /// this+0x40
    pub speed: f32,
    /// this+0x54 original texture name (restored on delete)
    pub orig_texture: Option<String>,
    /// this+0x58 current frame
    pub cur_frame: u32,
    /// this+0x5c connect kind: 0 none, 0xf EF3dObj clump, 0x18 static world
    pub target: u32,
    /// P26 R31: in the connected EF3dObj's +0x60 texture-anim list (Play on a 0xf node adds it,
    /// Stop removes it); EF3dObj::Render calls ApplyText/RestoreText for the list
    pub played: bool,
    pub events: Vec<AnimEvent>,
}

impl EFTextureAnimNode {
    /// ctor + Initialize(params): params->+0x18 texture name (the original texture), duration =
    /// frames[count-1].time. quirk: count 0 reads frames[-1] (here: 0.0).
    pub fn new(res: Arc<EFTextureAnimRes>, user: u32, orig_texture: Option<String>) -> Self {
        let duration = res.frames.last().map_or(0.0, |f| f.1);
        EFTextureAnimNode { user, excl: 0, obj: 0, res, state: state::STOPPED, time: 0.0, duration, speed: 1.0, orig_texture, cur_frame: 0, target: 0, played: false, events: Vec::new() }
    }

    /// EFTextureAnimNode::Connect(EFInstance*): `kind` = instance type (+8 >> 24), `found` = the
    /// atomic/material search found the original texture.
    pub fn Connect(&mut self, obj: u32, kind: u32, found: bool) {
        if !found {
            return;
        }
        if kind == 0xf {
            self.target = 0xf;
            self.obj = obj;
        } else if kind == 0x18 {
            self.target = 0x18;
        }
    }

    /// EFTextureAnimNode::Play
    pub fn Play(&mut self, flags: u32, _f: f32) {
        if self.target != 0 {
            self.state = flags; // EFAnimNode::Play
            if self.target == 0xf {
                self.played = true;
            }
            self.cur_frame = 0;
        }
    }

    /// EFTextureAnimNode::Stop (the tree detach is the mgr's business)
    pub fn Stop(&mut self) {
        if self.state != state::STOPPED {
            self.played = false;
            self.state = state::STOPPED;
            self.time = 0.0;
            self.events.push(AnimEvent::Notify { code: 0, user: self.user, arg: 0 });
        }
    }

    /// texture of the current frame (what RpMaterialSetTexture / ApplyText would set)
    pub fn current_texture(&self) -> Option<&str> {
        self.res.frames.get(self.cur_frame as usize).map(|f| f.0.as_str())
    }

    /// EFTextureAnimNode::Update(dt)
    pub fn Update(&mut self, dt: f32) -> u32 {
        use state::*;
        if self.state & STOPPED == 0 && 0.0 < self.duration {
            self.time += dt * self.speed;
            self.state &= !WRAPPED;
            if self.duration <= self.time {
                if self.state & LOOP != 0 {
                    while self.duration < self.time {
                        self.time -= self.duration;
                        self.state |= WRAPPED;
                    }
                } else if self.state & PLAY_ONCE != 0 {
                    self.time = self.duration;
                    self.state |= END_REACHED;
                }
            } else if self.time <= 0.0 {
                if self.state & LOOP != 0 {
                    while self.time < 0.0 {
                        self.time += self.duration;
                        self.state |= WRAPPED;
                    }
                } else if self.state & PLAY_ONCE != 0 {
                    self.time = 0.0;
                    self.state |= END_REACHED;
                }
            }
            if self.state & WRAPPED != 0 {
                self.state |= END_REACHED;
            }
        }
        // after a wrap the frame search restarts at 0, else from the current frame
        let mut i = if self.state & WRAPPED == 0 { self.cur_frame } else { 0 };
        // quirk: unbounded search in C++; times never exceed the last frame's time.
        while (i as usize) < self.res.frames.len() && self.res.frames[i as usize].1 < self.time {
            i += 1;
        }
        self.cur_frame = i;
        if self.state & PLAY_ONCE != 0
            && ((self.speed < 0.0 && self.time == 0.0) || (0.0 < self.speed && self.time == self.duration))
        {
            self.state |= FINISHED;
        }
        if self.state & END_REACHED != 0 {
            self.events.push(AnimEvent::Notify { code: 1, user: self.user, arg: self.user });
            self.state &= !END_REACHED;
        }
        0
    }
}
