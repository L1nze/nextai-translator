import { AbstractFreeMT, fetchJson, googleTargetLang, googleTk, runFreeTranslation } from './free-mt'
import { IMessageRequest } from './interfaces'

// Free Google Translate web endpoint (translate.googleapis.com, no API key):
// GET /translate_a/single?client=gtx&dt=t&tk=<signature>&q=<text>
// Response data[0] is an array of [translated, original, ...] segments.
export function parseGoogleFreeText(data: unknown): string {
    const rows = (data as unknown[][])?.[0]
    if (!Array.isArray(rows)) {
        throw new Error('Google Translate returned an unparsable result')
    }
    let out = ''
    for (const row of rows) {
        if (Array.isArray(row) && typeof row[0] === 'string') {
            out += row[0]
        }
    }
    if (!out.trim()) {
        throw new Error('Google Translate returned no result')
    }
    return out
}

export class GoogleTranslate extends AbstractFreeMT {
    readonly serviceName = 'Google Translate'

    async sendMessage(req: IMessageRequest): Promise<void> {
        await runFreeTranslation(req, {
            label: this.serviceName,
            chunk: 1400,
            max: 10000,
            makeChunkTranslator: async () => {
                const target = googleTargetLang(req.toLang)
                return async (chunk) => {
                    const url =
                        'https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=' +
                        encodeURIComponent(target) +
                        '&dt=t&tk=' +
                        googleTk(chunk) +
                        '&q=' +
                        encodeURIComponent(chunk)
                    const data = await fetchJson(this.serviceName, url, { method: 'GET' })
                    return parseGoogleFreeText(data)
                }
            },
        })
    }
}
