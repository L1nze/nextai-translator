import { IconBaseProps } from 'react-icons'

// Xiaomi MiMo — M glyph evoking the MI mark, drawn inline so it follows the
// current theme color without shipping a brand asset.
export function MiMoIcon(props: IconBaseProps) {
    const size = props.size ?? '1em'
    return (
        <svg width={size} height={size} viewBox='0 0 24 24' fill='currentColor' aria-hidden='true'>
            <path d='M4 6v12h3.2v-7.4l4.8 5.8 4.8-5.8V18H20V6l-8 9.6z' />
        </svg>
    )
}
