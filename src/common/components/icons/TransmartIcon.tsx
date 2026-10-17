import { IconBaseProps } from 'react-icons'

// Tencent Transmart (腾讯交互翻译) — chat bubble with a "T" glyph, drawn
// inline so it follows the current theme color without a brand asset.
export function TransmartIcon(props: IconBaseProps) {
    const size = props.size ?? '1em'
    return (
        <svg
            width={size}
            height={size}
            viewBox='0 0 24 24'
            fill='none'
            stroke='currentColor'
            strokeWidth='2.2'
            strokeLinecap='round'
            strokeLinejoin='round'
            aria-hidden='true'
        >
            <path d='M4 5h16v11H9l-5 4V5z' />
            <path d='M8 9h8' />
            <path d='M12 9v5' />
        </svg>
    )
}
