//! Profile resolution and transform caching.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use core::fmt;

use tpt_colour_core::space::{d50_to_d65, d65_to_d50};
use tpt_colour_icc::{ColourTransform, IccError, IccProfile, RenderingIntent};

use crate::gamut::Gamut;
use crate::map::map_to_gamut;

/// Errors from the colour manager.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ManageError {
    /// No profile is registered under this name.
    UnknownProfile(String),
    /// The profile or transform could not be built.
    Icc(IccError),
}

impl From<IccError> for ManageError {
    fn from(e: IccError) -> Self {
        Self::Icc(e)
    }
}

impl fmt::Display for ManageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProfile(name) => write!(f, "no profile registered as {name:?}"),
            Self::Icc(e) => write!(f, "{e}"),
        }
    }
}

/// A transform with its destination gamut, ready to run.
#[derive(Clone, Debug, PartialEq)]
pub struct ManagedTransform {
    inner: ColourTransform,
    gamut: Option<Gamut>,
}

impl ManagedTransform {
    /// Transforms one device colour, gamut-mapping perceptually when the destination has a boundary.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] if either slice has the wrong length.
    pub fn transform(&self, src: &[f32], dst: &mut [f32]) -> Result<(), IccError> {
        let xyz = self.inner.device_to_pcs(src)?;
        let xyz = match &self.gamut {
            Some(gamut) => d65_to_d50(map_to_gamut(gamut, d50_to_d65(xyz))),
            None => xyz,
        };
        self.inner.pcs_to_device(xyz, dst)
    }

    /// Transforms one RGB triple.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] unless both profiles are three-channel.
    ///
    /// Without a destination gamut there is nothing to map, so this uses the ICC single-colour
    /// path (f64). With a gamut, the mapping runs in f32 as usual.
    pub fn transform_rgb(&self, rgb: [f32; 3]) -> Result<[f32; 3], IccError> {
        if self.gamut.is_none() {
            return self.inner.transform_rgb(rgb);
        }
        let mut out = [0.0_f32; 3];
        self.transform(&rgb, &mut out)?;
        Ok(out)
    }

    /// The underlying ICC transform.
    #[must_use]
    pub fn inner(&self) -> &ColourTransform {
        &self.inner
    }

    /// The destination gamut used for perceptual mapping, if any.
    #[must_use]
    pub fn gamut(&self) -> Option<&Gamut> {
        self.gamut.as_ref()
    }
}

/// A cached transform and the tick at which it was last used.
#[derive(Debug)]
struct Cached {
    transform: Arc<ManagedTransform>,
    last_used: u64,
}

/// One cached transform per rendering intent, indexed by [`intent_code`].
type IntentSlots = [Option<Cached>; 4];

/// Registers profiles by name and caches the transforms built from them.
///
/// Built-in `"srgb"` and `"display-p3"` profiles are registered by [`ColourManager::new`].
/// Re-registering a name drops every cached transform that uses it.
///
/// The cache is keyed by source name, then destination name, so a lookup by `&str` does not
/// allocate. Only a cache miss copies the names.
#[derive(Debug)]
pub struct ColourManager {
    profiles: BTreeMap<String, Arc<IccProfile>>,
    cache: BTreeMap<String, BTreeMap<String, IntentSlots>>,
    capacity: usize,
    clock: u64,
}

impl Default for ColourManager {
    fn default() -> Self {
        Self {
            profiles: BTreeMap::new(),
            cache: BTreeMap::new(),
            capacity: usize::MAX,
            clock: 0,
        }
    }
}

impl ColourManager {
    /// Creates a manager with the built-in sRGB and Display P3 profiles registered. The cache is
    /// unbounded; use [`ColourManager::with_capacity`] to cap it.
    #[must_use]
    pub fn new() -> Self {
        let mut m = Self::default();
        m.register("srgb", IccProfile::srgb());
        m.register("display-p3", IccProfile::display_p3());
        m
    }

    /// Creates a manager whose cache holds at most `max_transforms` transforms. When a new one
    /// would exceed the cap, the least recently used transform is dropped. A cap of zero is
    /// treated as one.
    ///
    /// # Example
    ///
    /// ```
    /// use tpt_colour_manage::ColourManager;
    /// use tpt_colour_icc::RenderingIntent;
    ///
    /// let mut colour = ColourManager::with_capacity(2);
    /// colour.transform("srgb", "display-p3", RenderingIntent::Perceptual)?;
    /// colour.transform("display-p3", "srgb", RenderingIntent::Perceptual)?;
    /// colour.transform("srgb", "srgb", RenderingIntent::Perceptual)?;
    /// assert_eq!(colour.cached_transforms(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn with_capacity(max_transforms: usize) -> Self {
        let mut m = Self::new();
        m.capacity = max_transforms.max(1);
        m
    }

    /// Registers a profile under `name`, replacing any previous one.
    pub fn register(&mut self, name: &str, profile: IccProfile) {
        self.profiles.insert(String::from(name), Arc::new(profile));
        self.cache.remove(name);
        for destinations in self.cache.values_mut() {
            destinations.remove(name);
        }
        self.cache
            .retain(|_, destinations| !destinations.is_empty());
    }

    /// Parses ICC bytes and registers the result under `name`.
    ///
    /// # Errors
    ///
    /// Returns the parse error if the bytes are not a supported profile.
    pub fn register_icc(&mut self, name: &str, bytes: &[u8]) -> Result<(), IccError> {
        let profile = IccProfile::parse(bytes)?;
        self.register(name, profile);
        Ok(())
    }

    /// The profile registered under `name`.
    #[must_use]
    pub fn profile(&self, name: &str) -> Option<&IccProfile> {
        self.profiles.get(name).map(Arc::as_ref)
    }

    /// Number of transforms currently cached.
    #[must_use]
    pub fn cached_transforms(&self) -> usize {
        self.cache
            .values()
            .flat_map(|destinations| destinations.values())
            .flatten()
            .filter(|slot| slot.is_some())
            .count()
    }

    /// Returns the transform from `src` to `dst`, building and caching it on first use.
    ///
    /// Perceptual transforms gamut-map into the destination when it has a gamut boundary.
    /// Relative and absolute transforms clip. Black point compensation follows
    /// [`ColourTransform::new`]: implicit for perceptual and saturation to v4 destinations, and
    /// otherwise off. Use [`ColourTransform::with_black_point_compensation`] directly to force it.
    ///
    /// # Errors
    ///
    /// Returns [`ManageError::UnknownProfile`] if either name is not registered, or
    /// [`ManageError::Icc`] if the transform cannot be built.
    pub fn transform(
        &mut self,
        src: &str,
        dst: &str,
        intent: RenderingIntent,
    ) -> Result<Arc<ManagedTransform>, ManageError> {
        let slot = usize::from(intent_code(intent));
        self.clock += 1;
        let now = self.clock;
        if let Some(cached) = self
            .cache
            .get_mut(src)
            .and_then(|destinations| destinations.get_mut(dst))
            .and_then(|slots| slots[slot].as_mut())
        {
            cached.last_used = now;
            return Ok(Arc::clone(&cached.transform));
        }
        let src_profile = self.resolve(src)?;
        let dst_profile = self.resolve(dst)?;
        let inner = ColourTransform::new(&src_profile, &dst_profile, intent)?;
        let gamut = match intent {
            RenderingIntent::Perceptual => Gamut::from_profile(&dst_profile),
            _ => None,
        };
        let managed = Arc::new(ManagedTransform { inner, gamut });
        if self.cached_transforms() >= self.capacity {
            self.evict_least_recently_used();
        }
        self.cache
            .entry(String::from(src))
            .or_default()
            .entry(String::from(dst))
            .or_default()[slot] = Some(Cached {
            transform: Arc::clone(&managed),
            last_used: now,
        });
        Ok(managed)
    }

    /// Removes the cached transform with the oldest `last_used`, and any empty maps around it.
    fn evict_least_recently_used(&mut self) {
        let oldest = self
            .cache
            .iter()
            .flat_map(|(src, destinations)| {
                destinations.iter().flat_map(move |(dst, slots)| {
                    slots.iter().enumerate().filter_map(move |(slot, entry)| {
                        entry
                            .as_ref()
                            .map(|c| (c.last_used, src.clone(), dst.clone(), slot))
                    })
                })
            })
            .min_by_key(|(last_used, ..)| *last_used);
        let Some((_, src, dst, slot)) = oldest else {
            return;
        };
        if let Some(destinations) = self.cache.get_mut(&src) {
            if let Some(slots) = destinations.get_mut(&dst) {
                slots[slot] = None;
                if slots.iter().all(Option::is_none) {
                    destinations.remove(&dst);
                }
            }
            if destinations.is_empty() {
                self.cache.remove(&src);
            }
        }
    }

    fn resolve(&self, name: &str) -> Result<Arc<IccProfile>, ManageError> {
        self.profiles
            .get(name)
            .map(Arc::clone)
            .ok_or_else(|| ManageError::UnknownProfile(String::from(name)))
    }
}

fn intent_code(intent: RenderingIntent) -> u8 {
    match intent {
        RenderingIntent::Perceptual => 0,
        RenderingIntent::RelativeColorimetric => 1,
        RenderingIntent::Saturation => 2,
        RenderingIntent::AbsoluteColorimetric => 3,
    }
}

impl core::error::Error for ManageError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Icc(e) => Some(e),
            Self::UnknownProfile(_) => None,
        }
    }
}

#[cfg(test)]
mod lru_tests {
    use alloc::sync::Arc;

    use super::ColourManager;
    use tpt_colour_icc::RenderingIntent;

    #[test]
    fn the_least_recently_used_transform_is_evicted() {
        let mut m = ColourManager::with_capacity(2);
        let intent = RenderingIntent::Perceptual;
        let a = m.transform("srgb", "display-p3", intent).unwrap();
        let _b = m.transform("display-p3", "srgb", intent).unwrap();
        // Touch `a`, so `b` is now the oldest.
        let a_again = m.transform("srgb", "display-p3", intent).unwrap();
        assert!(
            Arc::ptr_eq(&a, &a_again),
            "a hit returns the cached transform"
        );
        let _c = m.transform("srgb", "srgb", intent).unwrap();
        assert_eq!(m.cached_transforms(), 2);
        // `a` survived, so it is still the same allocation.
        let a_third = m.transform("srgb", "display-p3", intent).unwrap();
        assert!(
            Arc::ptr_eq(&a, &a_third),
            "the recently used entry was kept"
        );
    }

    #[test]
    fn an_unbounded_manager_keeps_everything() {
        let mut m = ColourManager::new();
        for intent in [
            RenderingIntent::Perceptual,
            RenderingIntent::RelativeColorimetric,
            RenderingIntent::Saturation,
            RenderingIntent::AbsoluteColorimetric,
        ] {
            m.transform("srgb", "display-p3", intent).unwrap();
        }
        assert_eq!(m.cached_transforms(), 4);
    }

    #[test]
    fn a_zero_cap_still_caches_one_transform() {
        let mut m = ColourManager::with_capacity(0);
        m.transform("srgb", "display-p3", RenderingIntent::Perceptual)
            .unwrap();
        m.transform("display-p3", "srgb", RenderingIntent::Perceptual)
            .unwrap();
        assert_eq!(m.cached_transforms(), 1);
    }
}
