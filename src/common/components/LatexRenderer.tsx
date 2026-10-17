import 'katex/dist/katex.min.css'
import Latex from 'react-latex-next'

// Loaded lazily from the Translator so KaTeX's CSS/fonts only load for the
// rare action configured with the LaTeX rendering format.
export default function LatexRenderer({ children }: { children: string }) {
    return <Latex>{children}</Latex>
}
