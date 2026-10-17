import { IconBaseProps } from 'react-icons'

// Zhipu AI (智谱 GLM) — bold Z monogram, drawn inline so it follows the
// current theme color without shipping a brand asset.
export function ZhipuIcon(props: IconBaseProps) {
    const size = props.size ?? '1em'
    return (
        <svg width={size} height={size} viewBox='0 0 24 24' fill='currentColor' aria-hidden='true'>
            <path d='M6 4h13v3.6L13.1 15H19v5H6v-3.6L11.9 8H6z' />
        </svg>
    )
}
