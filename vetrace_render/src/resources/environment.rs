use serde::{Deserialize, Serialize};

use crate::components::CubemapHandle;

/// Scene-wide sky and image-based-lighting selection.
///
/// `primary` and `secondary` are sampled in linear light and blended using
/// `transition`, allowing day/night and level-state changes without a visible
/// cubemap pop.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentCubemap {
    pub enabled: bool,
    pub primary: Option<CubemapHandle>,
    pub secondary: Option<CubemapHandle>,
    pub transition: f32,
    pub intensity: f32,
    pub rotation_radians: f32,
    pub draw_sky: bool,
    pub diffuse_ibl: bool,
    pub specular_ibl: bool,
}

impl Default for EnvironmentCubemap {
    fn default() -> Self {
        Self {
            enabled: false,
            primary: None,
            secondary: None,
            transition: 0.0,
            intensity: 1.0,
            rotation_radians: 0.0,
            draw_sky: true,
            diffuse_ibl: false,
            specular_ibl: true,
        }
    }
}


impl EnvironmentCubemap {
    /// Starts a smooth transition while keeping the current cubemap alive.
    pub fn begin_transition(&mut self, next: CubemapHandle) {
        self.enabled = true;
        match self.primary {
            None => {
                self.primary = Some(next);
                self.secondary = None;
                self.transition = 0.0;
            }
            Some(current) if current == next => {
                self.secondary = None;
                self.transition = 0.0;
            }
            Some(_) => {
                self.secondary = Some(next);
                self.transition = 0.0;
            }
        }
    }

    /// Advances an active crossfade and promotes the target when it completes.
    /// Returns `true` only on the frame where the transition completes.
    pub fn advance_transition(&mut self, delta_seconds: f32, duration_seconds: f32) -> bool {
        let Some(next) = self.secondary else {
            self.transition = 0.0;
            return false;
        };
        if duration_seconds <= 0.0 {
            self.primary = Some(next);
            self.secondary = None;
            self.transition = 0.0;
            return true;
        }
        self.transition = (self.transition + delta_seconds.max(0.0) / duration_seconds).clamp(0.0, 1.0);
        if self.transition < 1.0 {
            return false;
        }
        self.primary = Some(next);
        self.secondary = None;
        self.transition = 0.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossfade_promotes_secondary() {
        let mut environment = EnvironmentCubemap {
            enabled: true,
            primary: Some(CubemapHandle(1)),
            ..EnvironmentCubemap::default()
        };
        environment.begin_transition(CubemapHandle(2));
        assert_eq!(environment.secondary, Some(CubemapHandle(2)));
        assert!(!environment.advance_transition(0.25, 1.0));
        assert_eq!(environment.transition, 0.25);
        assert!(environment.advance_transition(0.75, 1.0));
        assert_eq!(environment.primary, Some(CubemapHandle(2)));
        assert_eq!(environment.secondary, None);
        assert_eq!(environment.transition, 0.0);
    }
}
