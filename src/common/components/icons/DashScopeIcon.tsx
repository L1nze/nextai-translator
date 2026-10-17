import { IconBaseProps } from 'react-icons'

// DashScope (通义千问) — spark glyph evoking the Tongyi mark, drawn inline so
// it follows the current theme color without shipping a brand asset.
export function DashScopeIcon(props: IconBaseProps) {
    const size = props.size ?? '1em'
    return (
        <svg width={size} height={size} viewBox='0 0 24 24' fill='currentColor' aria-hidden='true'>
            <path d='M11 2.5c.9 3.6 1.9 4.6 5.5 5.5-3.6.9-4.6 1.9-5.5 5.5-.9-3.6-1.9-4.6-5.5-5.5 3.6-.9 4.6-1.9 5.5-5.5z' />
            <path d='M17.5 12.5c.6 2.4 1.3 3.1 3.7 3.7-2.4.6-3.1 1.3-3.7 3.7-.6-2.4-1.3-3.1-3.7-3.7 2.4-.6 3.1-1.3 3.7-3.7z' />
            <path d='M7.5 15.5c.5 2 .8 2.6 2.8 3.1-2 .5-2.6.8-3.1 2.8-.5-2-.8-2.6-2.8-3.1 2-.5 2.6-.8 3.1-2.8z' />
        </svg>
    )
}
