//! Resolve CLI / env resource policy flags.
//!
//! Precedence (highest first):
//!   1. `--resource-policy` (explicit, wins over everything)
//!   2. `--full-render` / `--text-only` (shorthands)
//!   3. `FLASHBROWSE_RESOURCE_POLICY` (env)
//!   4. `agent-default`
//!
//! The shorthands and the explicit flag are mutually exclusive when they
//! disagree, so a conflicting invocation fails loudly instead of silently
//! picking one.

use flashbrowse_core::ResourcePolicy;

/// Build the effective policy from global flags.
pub fn resolve_resource_policy(
    text_only: bool,
    full_render: bool,
    explicit: Option<&str>,
    env: Option<&str>,
) -> anyhow::Result<ResourcePolicy> {
    if text_only && full_render {
        anyhow::bail!("--text-only and --full-render are mutually exclusive");
    }

    // An explicit --resource-policy always wins, but it must not contradict
    // a shorthand the user also passed.
    let explicit_policy = match explicit {
        Some(s) => Some(s.parse::<ResourcePolicy>().map_err(anyhow::Error::msg)?),
        None => None,
    };
    let shorthand = if full_render {
        Some(ResourcePolicy::FullRender)
    } else if text_only {
        Some(ResourcePolicy::TextOnly)
    } else {
        None
    };
    if let (Some(a), Some(b)) = (explicit_policy, shorthand) {
        if a != b {
            anyhow::bail!(
                "--resource-policy {} conflicts with the {} shorthand",
                a.as_str(),
                b.as_str()
            );
        }
    }
    if let Some(p) = explicit_policy {
        return Ok(p);
    }
    if let Some(p) = shorthand {
        return Ok(p);
    }
    if let Some(s) = env {
        return s.parse::<ResourcePolicy>().map_err(anyhow::Error::msg);
    }
    Ok(ResourcePolicy::AgentDefault)
}

#[cfg(test)]
mod tests {
    use super::resolve_resource_policy;
    use flashbrowse_core::ResourcePolicy;

    #[test]
    fn explicit_flag_wins_over_env() {
        let p = resolve_resource_policy(false, false, Some("text-only"), Some("full-render"))
            .expect("resolve");
        assert_eq!(p, ResourcePolicy::TextOnly);
    }

    #[test]
    fn shorthand_conflicting_with_explicit_is_an_error() {
        let err = resolve_resource_policy(false, true, Some("text-only"), None).unwrap_err();
        assert!(err.to_string().contains("conflicts"), "{err}");
    }

    #[test]
    fn matching_shorthand_and_explicit_is_fine() {
        let p = resolve_resource_policy(false, true, Some("full-render"), None).expect("resolve");
        assert_eq!(p, ResourcePolicy::FullRender);
    }

    #[test]
    fn env_is_used_when_no_flag() {
        let p = resolve_resource_policy(false, false, None, Some("full-render")).expect("resolve");
        assert_eq!(p, ResourcePolicy::FullRender);
    }

    #[test]
    fn default_is_agent_default() {
        let p = resolve_resource_policy(false, false, None, None).expect("resolve");
        assert_eq!(p, ResourcePolicy::AgentDefault);
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert!(resolve_resource_policy(false, false, Some("bogus"), None).is_err());
        assert!(resolve_resource_policy(false, false, None, Some("bogus")).is_err());
    }
}
