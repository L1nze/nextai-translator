import { IconBaseProps } from 'react-icons'

// OpenCode — terminal prompt glyph (chevron + cursor), drawn inline so it
// follows the current theme color without shipping a brand asset.
export function OpenCodeIcon(props: IconBaseProps) {
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
            strokeLinejoin='round'
            aria-hidden='true'
        >
            <path d='M6 5l6 7-6 7' />
            <path d='M15 19h4' />
        </svg>
    )
}
