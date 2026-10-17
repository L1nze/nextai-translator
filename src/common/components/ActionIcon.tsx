import { Suspense, createElement, lazy, type ReactNode } from 'react'
import type { IconType } from 'react-icons'
import { MdOutlineGTranslate, MdPalette, MdOutlineSummarize, MdOutlineAnalytics, MdCode } from './local-md-icons'

// The icons of the built-in actions come from a local module so the common
// case never pulls the full Material icon set (a named import from
// 'react-icons/md' would re-attach the window to that ~2MB index). Anything a
// custom action picked in the icon picker falls back to the lazy registry.
const builtinActionIcons: Record<string, IconType> = {
    MdOutlineGTranslate,
    MdPalette,
    MdOutlineSummarize,
    MdOutlineAnalytics,
    MdCode,
}

const MdIcon = lazy(() => import('./md-icon-registry').then((m) => ({ default: m.MdIconRegistry })))

export function ActionIcon({ name, size }: { name?: string; size?: number }): ReactNode {
    if (!name) {
        return null
    }
    const builtinIcon = builtinActionIcons[name]
    if (builtinIcon) {
        return createElement(builtinIcon, { size })
    }
    return (
        <Suspense fallback={<span style={{ display: 'inline-block', width: size, height: size }} />}>
            <MdIcon name={name} size={size} />
        </Suspense>
    )
}
