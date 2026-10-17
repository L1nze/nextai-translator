import { useEffect, useState } from 'react'
import { useTheme } from '../hooks/useTheme'
import { isTauri } from '../utils'

// Only the standalone utility windows are undecorated on the Rust side;
// other surfaces (the translator popup, the settings overlay embedded in
// it) must not grow window controls.
const WINDOW_LABELS_WITH_CONTROLS = new Set(['settings', 'history'])

const ICONS = {
    minimize: 'M0 4.5h10v1H0z',
    maximize: 'M0 0h10v10H0zM1 1v8h8V1z',
    restore: 'M2 0h8v8H2zM0 2h8v8H0z',
    close: 'M0 0h1.4L5 3.6 8.6 0H10v1.4L6.4 5 10 8.6V10H8.6L5 6.4 1.4 10H0V8.6L3.6 5 0 1.4z',
} as const

function CaptionButton(props: { iconPath: string; label: string; danger?: boolean; onClick: () => void }) {
    const { themeType } = useTheme()
    const [hover, setHover] = useState(false)

    const background = hover
        ? props.danger
            ? '#e81123'
            : themeType === 'dark'
            ? 'rgba(255, 255, 255, 0.1)'
            : 'rgba(0, 0, 0, 0.07)'
        : 'transparent'
    const iconColor = hover && props.danger ? '#ffffff' : themeType === 'dark' ? '#cbd5e1' : '#334155'

    return (
        <button
            type='button'
            aria-label={props.label}
            title={props.label}
            style={{
                display: 'inline-flex',
                justifyContent: 'center',
                alignItems: 'center',
                width: '44px',
                height: '34px',
                border: 'none',
                padding: 0,
                margin: 0,
                outline: 'none',
                background,
                cursor: 'default',
                transition: 'background 0.12s ease',
            }}
            onMouseEnter={() => setHover(true)}
            onMouseLeave={() => setHover(false)}
            onClick={props.onClick}
        >
            <svg width='10' height='10' viewBox='0 0 10 10' aria-hidden='true'>
                <path d={props.iconPath} fill={iconColor} />
            </svg>
        </button>
    )
}

/**
 * Minimize / maximize / close controls for the undecorated utility
 * windows (Settings, History). Renders nothing outside the Tauri app or
 * in windows that keep their own chrome, so it can be mounted from shared
 * components safely.
 */
export function WindowControls() {
    const [visible, setVisible] = useState(false)
    const [isMaximized, setIsMaximized] = useState(false)

    useEffect(() => {
        if (!isTauri()) {
            return undefined
        }
        let unlisten: (() => void) | undefined
        let cancelled = false
        // Dynamic import keeps @tauri-apps/api out of the browser
        // extension bundles.
        import('@tauri-apps/api/webviewWindow').then(({ WebviewWindow }) => {
            const appWindow = WebviewWindow.getCurrent()
            if (!WINDOW_LABELS_WITH_CONTROLS.has(appWindow.label)) {
                return
            }
            setVisible(true)
            appWindow
                .isMaximized()
                .then((maximized) => {
                    if (!cancelled) {
                        setIsMaximized(maximized)
                    }
                })
                .catch(console.error)
            appWindow
                .onResized(() => {
                    appWindow
                        .isMaximized()
                        .then((maximized) => {
                            if (!cancelled) {
                                setIsMaximized(maximized)
                            }
                        })
                        .catch(console.error)
                })
                .then((cb) => {
                    unlisten = cb
                })
                .catch(console.error)
        })
        return () => {
            cancelled = true
            unlisten?.()
        }
    }, [])

    if (!visible) {
        return null
    }

    const minimize = () => {
        import('@tauri-apps/api/webviewWindow').then(({ WebviewWindow }) => {
            WebviewWindow.getCurrent().minimize().catch(console.error)
        })
    }
    const toggleMaximize = () => {
        import('@tauri-apps/api/webviewWindow').then(({ WebviewWindow }) => {
            WebviewWindow.getCurrent().toggleMaximize().catch(console.error)
        })
    }
    const close = () => {
        import('@tauri-apps/api/webviewWindow').then(({ WebviewWindow }) => {
            WebviewWindow.getCurrent().close().catch(console.error)
        })
    }

    return (
        <div
            style={{
                position: 'fixed',
                top: 0,
                right: 0,
                zIndex: 1000,
                display: 'flex',
                WebkitUserSelect: 'none',
                userSelect: 'none',
            }}
        >
            <CaptionButton iconPath={ICONS.minimize} label='Minimize' onClick={minimize} />
            <CaptionButton
                iconPath={isMaximized ? ICONS.restore : ICONS.maximize}
                label={isMaximized ? 'Restore' : 'Maximize'}
                onClick={toggleMaximize}
            />
            <CaptionButton iconPath={ICONS.close} label='Close' danger onClick={close} />
        </div>
    )
}
