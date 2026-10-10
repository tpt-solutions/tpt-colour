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

/// Cache key: source name, destination name and intent.
type CacheKey = (String, String, u8);

/// Registers profiles by name and caches the transforms built from them.
///
/// Built-in `"srgb"` and `"display-p3"` profiles are registered by [`ColourManager::new`].
/// Re-registering a name drops every cached transform that uses it.
#[derive(Debug, Default)]
pub struct ColourManager {
    profiles: BTreeMap<String, Arc<IccProfile>>,
    cache: BTreeMap<CacheKey, Arc<ManagedTransform>>,
}

impl ColourManager {
    /// Creates a manager with the built-in sRGB and Display P3 profiles registered.
    #[must_use]
    pub fn new() -> Self {
        let mut m = Self::default();
        m.register("srgb", IccProfile::srgb());
        m.register("display-p3", IccProfile::display_p3());
        m
    }

    /// Registers a profile under `name`, replacing any previous one.
    pub fn register(&mut self, name: &str, profile: IccProfile) {
        self.profiles.insert(String::from(name), Arc::new(profile));
        self.cache
            .retain(|(src, dst, _), _| src != name && dst != name);
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
        self.cache.len()
    }

    /// Returns the transform from `src` to `dst`, building and caching it on first use.
    ///
    /// Perceptual transforms gamut-map into the destination when it is a matrix profile.
    /// Relative and absolute transforms clip. Black point compensation applies to all but
    /// absolute.
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
        let key = (String::from(src), String::from(dst), intent_code(intent));
        if let Some(t) = self.cache.get(&key) {
            return Ok(Arc::clone(t));
        }
        let src_profile = self.resolve(src)?;
        let dst_profile = self.resolve(dst)?;
        let inner =
            ColourTransform::with_black_point_compensation(&src_profile, &dst_profile, intent)?;
        let gamut = match intent {
            RenderingIntent::Perceptual => Gamut::from_profile(&dst_profile),
            _ => None,
        };
        let managed = Arc::new(ManagedTransform { inner, gamut });
        self.cache.insert(key, Arc::clone(&managed));
        Ok(managed)
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

impl core::error::Error for ManageError {}
