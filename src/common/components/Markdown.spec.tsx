import { describe, expect, it, vi } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'

vi.mock('./CodeBlock', () => ({ CodeBlock: () => null }))
vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key, i18n: { language: 'en' } }),
}))
vi.mock('../hooks/useTheme', async () => {
    const { LightTheme } = await import('baseui-sd/themes')
    return { useTheme: () => ({ theme: LightTheme, themeType: 'light' }) }
})
vi.mock('../hooks/useSettings', () => ({
    useSettings: () => ({ settings: { tts: {} }, isSettingsLoading: false }),
}))
import { Markdown } from './Markdown'

describe('GFM table rendering', () => {
    it('renders pipe tables as HTML tables instead of plain paragraph text', () => {
        const tableMarkdown = '| 代码 | 说明 |\n| --- | --- |\n| diff | 差值计数 |'
        const html = renderToStaticMarkup(<Markdown>{tableMarkdown}</Markdown>)
        expect(html).toContain('<table')
        expect(html).toContain('<th')
        expect(html).toContain('<td')
        expect(html).not.toContain('|')
    })
})

describe('GFM table styling', () => {
    const tableMarkdown = '| 代码 | 说明 |\n| --- | --- |\n| diff | 差值计数 |'

    it('draws cell borders, centers content, and bolds/darkens the header row', () => {
        const html = renderToStaticMarkup(<Markdown>{tableMarkdown}</Markdown>)
        expect(html).toContain('border-collapse:collapse')
        expect(html).toMatch(/<th[^>]*font-weight:600/)
        expect(html).toMatch(/<th[^>]*background-color:/)
        expect(html).toMatch(/<td[^>]*text-align:center/)
        expect(html).toMatch(/<td[^>]*vertical-align:middle/)
        expect(html).toMatch(/<td[^>]*word-break:break-word/)
    })
})
