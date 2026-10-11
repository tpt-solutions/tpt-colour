//! Serde round trips for the public types, behind the `serde` feature.
#![cfg(feature = "serde")]

use core::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tpt_colour::composite::{BlendMode, PorterDuff};
use tpt_colour::icc::{RenderingIntent, Step};
use tpt_colour::vision::Deficiency;

fn round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = serde_json::to_string(value).expect("serialises");
    let back: T = serde_json::from_str(&json).expect("deserialises");
    assert_eq!(&back, value, "{json}");
}

#[test]
fn enums_round_trip() {
    round_trip(&RenderingIntent::AbsoluteColorimetric);
    round_trip(&PorterDuff::Xor);
    round_trip(&BlendMode::Luminosity);
    round_trip(&Deficiency::Tritanopia);
    round_trip(&Step::BlackPointCompensation);
    round_trip(&Step::Clut {
        grid: vec![17, 17, 17],
        outputs: 3,
    });
}

#[test]
fn names_are_stable() {
    assert_eq!(
        serde_json::to_string(&RenderingIntent::Saturation).unwrap(),
        "\"Saturation\""
    );
    assert_eq!(
        serde_json::to_string(&BlendMode::HardLight).unwrap(),
        "\"HardLight\""
    );
}
