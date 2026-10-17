/* eslint-disable camelcase */
import { Suspense, lazy, type ComponentType, type LazyExoticComponent } from 'react'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

// Every window loads only its own chunk: they share one WebView asset origin,
// but each new webview only parses and executes the code it actually renders,
// which is what dominates how fast a window becomes interactive.
const windowsMap: Record<string, LazyExoticComponent<ComponentType>> = {
    translator: lazy(() => import('./windows/TranslatorWindow').then((m) => ({ default: m.TranslatorWindow }))),
    action_manager: lazy(() =>
        import('./windows/ActionManagerWindow').then((m) => ({ default: m.ActionManagerWindow }))
    ),
    settings: lazy(() => import('./windows/SettingsWindow').then((m) => ({ default: m.SettingsWindow }))),
    thumb: lazy(() => import('./windows/ThumbWindow').then((m) => ({ default: m.ThumbWindow }))),
    updater: lazy(() => import('./windows/UpdaterWindow').then((m) => ({ default: m.UpdaterWindow }))),
    screenshot: lazy(() => import('./windows/ScreenshotWindow').then((m) => ({ default: m.ScreenshotWindow }))),
    history: lazy(() => import('./windows/HistoryWindow').then((m) => ({ default: m.HistoryWindow }))),
    inline_lookup: lazy(() => import('./windows/InlineLookupWindow').then((m) => ({ default: m.InlineLookupWindow }))),
    quick_translator: lazy(() =>
        import('./windows/QuickTranslatorWindow').then((m) => ({ default: m.QuickTranslatorWindow }))
    ),
    writing_indicator: lazy(() =>
        import('./windows/WritingIndicatorWindow').then((m) => ({ default: m.WritingIndicatorWindow }))
    ),
}

export function App() {
    const appWindow = WebviewWindow.getCurrent()
    const Window = windowsMap[appWindow.label]
    return <Suspense fallback={null}>{Window ? <Window /> : null}</Suspense>
}
