import { IconBaseProps } from 'react-icons'

// SiliconFlow (硅基流动) — flowing double-wave glyph, drawn inline so it
// follows the current theme color without shipping a brand asset.
export function SiliconFlowIcon(props: IconBaseProps) {
    const size = props.size ?? '1em'
    return (
        <svg
            width={size}
            height={size}
            viewBox='0 0 24 24'
            fill='none'
            stroke='currentColor'
            strokeWidth='2.4'
            strokeLinecap='round'
            aria-hidden='true'
        >
            <path d='M3 8c3-4 6-4 9 0s6 4 9 0' />
            <path d='M3 16c3-4 6-4 9 0s6 4 9 0' />
        </svg>
    )
}
