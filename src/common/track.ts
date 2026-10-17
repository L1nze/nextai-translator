// Fork note: upstream telemetry (Aptabase) was removed; this no-op keeps
// the call sites intact without shipping events anywhere.
// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function trackEvent(_event: string, _props?: Record<string, unknown>) {
    // intentionally empty
}
