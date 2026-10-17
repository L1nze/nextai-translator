import { createElement } from 'react'
import type { IconType } from 'react-icons'
import * as mdIcons from 'react-icons/md'

export const mdIconMap = mdIcons as Record<string, IconType>

// Kept in its own module (loaded on demand via ActionIcon) so the full
// Material icon set stays out of the translator window's initial bundle.
export function MdIconRegistry({ name, size }: { name: string; size?: number }) {
    const icon = (mdIcons as Record<string, IconType>)[name]
    if (!icon) {
        return null
    }
    return createElement(icon, { size })
}
