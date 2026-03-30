//! Shared freshness helpers for live project-status stamping.
//!
//! Maintainer note:
//! - Keep this module focused on reusable freshness assembly rules.
//! - Callers choose which live surfaces to sample and pass only the age values
//!   they already know.
//! - When a source can provide a real RFC3339 timestamp, `SystemTime`, or
//!   filesystem metadata timestamp, prefer the additive sample helper below so
//!   the caller still owns source attribution.

use crate::project_status::ProjectStatusFreshness;
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;
use std::fs::Metadata;
use std::time::SystemTime;

const PROJECT_STATUS_FRESHNESS_CURRENT: &str = "current";
const PROJECT_STATUS_FRESHNESS_STALE: &str = "stale";
const PROJECT_STATUS_FRESHNESS_UNKNOWN: &str = "unknown";
const PROJECT_STATUS_STALE_AGE_SECONDS: u64 = 7 * 24 * 60 * 60;

#[allow(dead_code)]
pub(crate) enum ProjectStatusFreshnessSample<'a> {
    AgeSeconds {
        source: &'a str,
        age_seconds: u64,
    },
    ObservedAtRfc3339 {
        source: &'a str,
        observed_at: &'a str,
    },
    ObservedAtSystemTime {
        source: &'a str,
        observed_at: SystemTime,
    },
    ObservedAtMetadata {
        source: &'a str,
        metadata: &'a Metadata,
    },
}

fn build_project_status_freshness(
    status: &str,
    source_count: usize,
    newest_age_seconds: Option<u64>,
    oldest_age_seconds: Option<u64>,
) -> ProjectStatusFreshness {
    ProjectStatusFreshness {
        status: status.to_string(),
        source_count,
        newest_age_seconds,
        oldest_age_seconds,
    }
}

#[allow(dead_code)]
fn age_seconds_from_system_time(now: SystemTime, observed_at: SystemTime) -> Option<u64> {
    now.duration_since(observed_at)
        .ok()
        .map(|age| age.as_secs())
}

#[allow(dead_code)]
fn age_seconds_from_observed_at(now: SystemTime, observed_at: &str) -> Option<u64> {
    let observed_at = DateTime::parse_from_rfc3339(observed_at).ok()?;
    let observed_at = observed_at.with_timezone(&Utc);
    age_seconds_from_system_time(now, observed_at.into())
}

#[allow(dead_code)]
fn age_seconds_from_metadata(now: SystemTime, metadata: &Metadata) -> Option<u64> {
    let observed_at = metadata.modified().ok()?;
    age_seconds_from_system_time(now, observed_at)
}

fn freshness_from_ages(ages: &[u64]) -> Option<(String, Option<u64>, Option<u64>)> {
    if ages.is_empty() {
        return None;
    }

    let newest_age_seconds = ages.iter().min().copied();
    let oldest_age_seconds = ages.iter().max().copied();
    let status = if oldest_age_seconds.unwrap_or(0) > PROJECT_STATUS_STALE_AGE_SECONDS {
        PROJECT_STATUS_FRESHNESS_STALE
    } else {
        PROJECT_STATUS_FRESHNESS_CURRENT
    };

    Some((status.to_string(), newest_age_seconds, oldest_age_seconds))
}

#[allow(dead_code)]
fn freshness_from_samples_at(
    now: SystemTime,
    samples: &[ProjectStatusFreshnessSample<'_>],
) -> Option<(String, Option<u64>, Option<u64>)> {
    if samples.is_empty() {
        return None;
    }

    let ages = samples
        .iter()
        .filter_map(|sample| match sample {
            ProjectStatusFreshnessSample::AgeSeconds { age_seconds, .. } => Some(*age_seconds),
            ProjectStatusFreshnessSample::ObservedAtRfc3339 { observed_at, .. } => {
                age_seconds_from_observed_at(now, observed_at)
            }
            ProjectStatusFreshnessSample::ObservedAtSystemTime { observed_at, .. } => {
                age_seconds_from_system_time(now, *observed_at)
            }
            ProjectStatusFreshnessSample::ObservedAtMetadata { metadata, .. } => {
                age_seconds_from_metadata(now, metadata)
            }
        })
        .collect::<Vec<_>>();

    freshness_from_ages(&ages)
        .or_else(|| Some((PROJECT_STATUS_FRESHNESS_CURRENT.to_string(), None, None)))
}

pub(crate) fn build_live_project_status_freshness(
    source_count: usize,
    ages: &[u64],
) -> ProjectStatusFreshness {
    let effective_source_count = if source_count > 0 {
        source_count
    } else {
        ages.len()
    };

    if effective_source_count == 0 {
        return build_project_status_freshness(PROJECT_STATUS_FRESHNESS_UNKNOWN, 0, None, None);
    }

    if let Some((status, newest_age_seconds, oldest_age_seconds)) = freshness_from_ages(ages) {
        return build_project_status_freshness(
            &status,
            effective_source_count,
            newest_age_seconds,
            oldest_age_seconds,
        );
    }

    build_project_status_freshness(
        PROJECT_STATUS_FRESHNESS_CURRENT,
        effective_source_count,
        None,
        None,
    )
}

#[allow(dead_code)]
pub(crate) fn build_live_project_status_freshness_from_samples(
    samples: &[ProjectStatusFreshnessSample<'_>],
) -> ProjectStatusFreshness {
    let effective_source_count = samples
        .iter()
        .map(|sample| match sample {
            ProjectStatusFreshnessSample::AgeSeconds { source, .. }
            | ProjectStatusFreshnessSample::ObservedAtRfc3339 { source, .. }
            | ProjectStatusFreshnessSample::ObservedAtSystemTime { source, .. }
            | ProjectStatusFreshnessSample::ObservedAtMetadata { source, .. } => *source,
        })
        .collect::<BTreeSet<_>>()
        .len();

    if effective_source_count == 0 {
        return build_project_status_freshness(PROJECT_STATUS_FRESHNESS_UNKNOWN, 0, None, None);
    }

    let freshness = freshness_from_samples_at(SystemTime::now(), samples).unwrap_or((
        PROJECT_STATUS_FRESHNESS_CURRENT.to_string(),
        None,
        None,
    ));
    build_project_status_freshness(
        &freshness.0,
        effective_source_count,
        freshness.1,
        freshness.2,
    )
}

pub(crate) fn build_live_project_status_freshness_from_source_count(
    source_count: usize,
) -> ProjectStatusFreshness {
    build_live_project_status_freshness(source_count, &[])
}

#[cfg(test)]
mod tests {
    use super::{
        build_live_project_status_freshness, build_live_project_status_freshness_from_samples,
        build_live_project_status_freshness_from_source_count, ProjectStatusFreshnessSample,
        PROJECT_STATUS_FRESHNESS_CURRENT, PROJECT_STATUS_FRESHNESS_STALE,
        PROJECT_STATUS_FRESHNESS_UNKNOWN, PROJECT_STATUS_STALE_AGE_SECONDS,
    };
    use chrono::{DateTime, Utc};
    use std::fs;
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;

    #[test]
    fn build_live_project_status_freshness_marks_source_count_only_current() {
        let freshness = build_live_project_status_freshness_from_source_count(2);

        assert_eq!(freshness.status, PROJECT_STATUS_FRESHNESS_CURRENT);
        assert_eq!(freshness.source_count, 2);
        assert_eq!(freshness.newest_age_seconds, None);
        assert_eq!(freshness.oldest_age_seconds, None);
    }

    #[test]
    fn build_live_project_status_freshness_marks_unknown_without_sources() {
        let freshness = build_live_project_status_freshness(0, &[]);

        assert_eq!(freshness.status, PROJECT_STATUS_FRESHNESS_UNKNOWN);
        assert_eq!(freshness.source_count, 0);
        assert_eq!(freshness.newest_age_seconds, None);
        assert_eq!(freshness.oldest_age_seconds, None);
    }

    #[test]
    fn build_live_project_status_freshness_uses_age_samples_when_available() {
        let freshness =
            build_live_project_status_freshness(0, &[15, PROJECT_STATUS_STALE_AGE_SECONDS + 1, 45]);

        assert_eq!(freshness.status, PROJECT_STATUS_FRESHNESS_STALE);
        assert_eq!(freshness.source_count, 3);
        assert_eq!(freshness.newest_age_seconds, Some(15));
        assert_eq!(
            freshness.oldest_age_seconds,
            Some(PROJECT_STATUS_STALE_AGE_SECONDS + 1)
        );
    }

    #[test]
    fn build_live_project_status_freshness_from_samples_uses_rfc3339_timestamps() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let now_rfc3339: String = DateTime::<Utc>::from(now).to_rfc3339();
        let old_rfc3339: String =
            DateTime::<Utc>::from(now - Duration::from_secs(PROJECT_STATUS_STALE_AGE_SECONDS + 1))
                .to_rfc3339();
        let freshness = {
            let samples = [
                ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                    source: "dashboard",
                    observed_at: &now_rfc3339,
                },
                ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                    source: "datasource",
                    observed_at: &old_rfc3339,
                },
            ];
            super::freshness_from_samples_at(now, &samples).unwrap()
        };

        assert_eq!(freshness.0, PROJECT_STATUS_FRESHNESS_STALE);
        assert_eq!(freshness.1, Some(0));
        assert_eq!(freshness.2, Some(PROJECT_STATUS_STALE_AGE_SECONDS + 1));
    }

    #[test]
    fn build_live_project_status_freshness_from_samples_uses_system_time_timestamps() {
        let freshness = build_live_project_status_freshness_from_samples(&[
            ProjectStatusFreshnessSample::ObservedAtSystemTime {
                source: "dashboard",
                observed_at: SystemTime::UNIX_EPOCH,
            },
            ProjectStatusFreshnessSample::ObservedAtSystemTime {
                source: "datasource",
                observed_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            },
        ]);

        assert_eq!(freshness.status, PROJECT_STATUS_FRESHNESS_STALE);
        assert_eq!(freshness.source_count, 2);
        let newest_age_seconds = freshness.newest_age_seconds.unwrap();
        let oldest_age_seconds = freshness.oldest_age_seconds.unwrap();
        assert_eq!(oldest_age_seconds, newest_age_seconds + 1);
    }

    #[test]
    fn build_live_project_status_freshness_from_samples_uses_metadata_modified_timestamps() {
        let tempdir = tempdir().unwrap();
        let path = tempdir.path().join("freshness.txt");
        fs::write(&path, "freshness").unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let freshness = build_live_project_status_freshness_from_samples(&[
            ProjectStatusFreshnessSample::ObservedAtMetadata {
                source: "dashboard",
                metadata: &metadata,
            },
        ]);

        assert_eq!(freshness.status, PROJECT_STATUS_FRESHNESS_CURRENT);
        assert_eq!(freshness.source_count, 1);
        assert!(freshness.newest_age_seconds.is_some());
        assert_eq!(freshness.newest_age_seconds, freshness.oldest_age_seconds);
    }

    #[test]
    fn build_live_project_status_freshness_from_samples_ignores_future_and_unparseable_timestamps_without_fabricating_age(
    ) {
        let freshness = build_live_project_status_freshness_from_samples(&[
            ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                source: "dashboard",
                observed_at: "2099-01-01T00:00:00Z",
            },
            ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                source: "datasource",
                observed_at: "still-not-a-timestamp",
            },
        ]);

        assert_eq!(freshness.status, PROJECT_STATUS_FRESHNESS_CURRENT);
        assert_eq!(freshness.source_count, 2);
        assert_eq!(freshness.newest_age_seconds, None);
        assert_eq!(freshness.oldest_age_seconds, None);
    }
}
