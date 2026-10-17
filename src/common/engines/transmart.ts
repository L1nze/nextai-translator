/* eslint-disable camelcase */
import { AbstractFreeMT, fetchJson, runFreeTranslation, transmartTargetLang } from './free-mt'
import { IMessageRequest } from './interfaces'

// Tencent Transmart interactive translation (transmart.qq.com web endpoint,
// no API key). client_key is the web client's public fixed value (same
// source as windingwind/zotero-pdf-translate).
const TRANSMART_CLIENT_KEY = 'browser-chrome-110.0.0-Mac OS-df4bd4c5-a65d-44b2-a40f-42f34f3535f2-1677486696487'

export function parseTransmartText(data: unknown): string {
    const rows = (data as { auto_translation?: unknown })?.auto_translation
    if (!Array.isArray(rows)) {
        throw new Error('Tencent Transmart returned an unparsable result')
    }
    const out = rows.filter(Boolean).join('\n').trim()
    if (!out) {
        throw new Error('Tencent Transmart returned no result')
    }
    return out
}

export class Transmart extends AbstractFreeMT {
    readonly serviceName = 'Tencent Transmart'

    async sendMessage(req: IMessageRequest): Promise<void> {
        await runFreeTranslation(req, {
            label: this.serviceName,
            chunk: 2000,
            max: 10000,
            makeChunkTranslator: async () => {
                const target = transmartTargetLang(req.toLang)
                return async (chunk) => {
                    const data = await fetchJson(this.serviceName, 'https://transmart.qq.com/api/imt', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json',
                            'User-Agent':
                                'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36',
                            'Referer': 'https://transmart.qq.com/zh-CN/index',
                        },
                        body: JSON.stringify({
                            header: { fn: 'auto_translation', client_key: TRANSMART_CLIENT_KEY },
                            type: 'plain',
                            model_category: 'normal',
                            source: { lang: 'auto', text_list: [chunk] },
                            target: { lang: target },
                        }),
                    })
                    return parseTransmartText(data)
                }
            },
        })
    }
}
