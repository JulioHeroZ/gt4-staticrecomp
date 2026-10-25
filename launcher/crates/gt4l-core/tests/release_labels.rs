#[path = "../build_support/release_label.rs"]
mod release_label;

use gt4l_core::{LAUNCHER_DISPLAY_VERSION, LAUNCHER_VERSION, RELEASE_LABEL, Version};
use gt4l_core::versions::{self, VersionFile};

#[test]
fn stable_and_candidate_display_labels_are_distinct() {
    assert_eq!(release_label::display_version("1.2.4", "").unwrap(), "1.2.4");
    assert_eq!(release_label::display_version("1.2.4", "rc1").unwrap(), "1.2.4-rc1");
    assert_eq!(release_label::display_version("1.2.4", "rc12").unwrap(), "1.2.4-rc12");
    for label in ["rc0", "rc01", "rc", "RC1", "beta1", "rc-1", "rc1.0", "rc1+test", "rc1\n", " rc1"] {
        assert!(release_label::display_version("1.2.4", label).is_err(), "invalid label accepted: {label:?}");
    }
}

#[test]
fn compiled_display_version_does_not_change_the_legacy_numeric_contract() {
    assert_eq!(LAUNCHER_DISPLAY_VERSION, release_label::display_version(LAUNCHER_VERSION, RELEASE_LABEL).unwrap());
    assert!(Version::parse(LAUNCHER_VERSION).is_some(), "the legacy updater must still receive a numeric version");
    let receipt = VersionFile::for_install(LAUNCHER_VERSION, gt4l_core::config::SCHEMA_VERSION);
    let round_trip = versions::parse(&versions::render(&receipt)).unwrap();
    assert_eq!(round_trip.gt4recomp, LAUNCHER_VERSION);
    assert!(versions::reconciled(&round_trip, LAUNCHER_VERSION, None).is_none());
    if !RELEASE_LABEL.is_empty() {
        assert_ne!(LAUNCHER_DISPLAY_VERSION, LAUNCHER_VERSION);
        assert!(Version::parse(LAUNCHER_DISPLAY_VERSION).is_none(), "do not feed candidate labels into the stable-only parser");
    }
}
