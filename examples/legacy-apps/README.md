# legacy-apps consumer example

Select the workload profile with `PELAGIAN_SHELL_PROFILE=legacy-apps`. It enables the optional Wine capability; it does not launch Wine or an application.

Install `profile.d/80-pbs.toml` as `/etc/pelagian-shell/profile.d/80-pbs.toml` only after validating PBS's real `app_id` and title behavior. The example is data-only: it supplies a window classification rule, not shell commands, launch logic, or task behavior.
