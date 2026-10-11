# The profile manager and its cache

`ColourManager` maps profile names to profiles, and caches the transforms built from them.

```rust,ignore
use tpt_colour::{ColourManager, RenderingIntent};

let mut colour = ColourManager::new();
colour.register("my-press", profile);            // an IccProfile you built or parsed
let t = colour.transform("srgb", "my-press", RenderingIntent::Perceptual)?;
```

## What is cached

The cache is keyed by source name, then destination name, then intent. A lookup with `&str`
allocates nothing. Only a miss copies the names and builds a transform.

## Invalidation

Re-registering a name drops every cached transform that uses that profile, as either source or
destination. Transforms already handed out stay valid, because they are reference-counted
(`Arc`), but new lookups build fresh ones.

## Bounding the cache

`ColourManager::new` caches without limit. For a long-running program that builds many
transforms, `ColourManager::with_capacity(n)` caps the cache at `n` transforms. When a new one would
exceed the cap, the least recently used transform is dropped. A hit refreshes its place.

```rust,ignore
let mut colour = ColourManager::with_capacity(32);
```

## Limits

- Lookups take `&mut self`. Sharing one manager between threads needs a lock around it. Interior
  mutability (`RwLock` or sharding) is not done yet, because it needs the `std` feature.
