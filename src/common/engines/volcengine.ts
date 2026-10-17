/* eslint-disable camelcase */
import { AbstractFreeMT, fetchJson, runFreeTranslation, volcTargetLang } from './free-mt'
import { IMessageRequest } from './interfaces'

// Volcengine Translate browser-extension endpoint (translate.volcengine.com,
// no API key): POST {source_language, target_language, text} -> {translation}.
export function parseVolcengineText(data: unknown): string {
    const out = (data as { translation?: unknown })?.translation
    if (!out || !String(out).trim()) {
        throw new Error('Volcengine Translate returned no result')
    }
    return String(out)
}

export class Volcengine extends AbstractFreeMT {
    readonly serviceName = 'Volcengine Translate'

    async sendMessage(req: IMessageRequest): Promise<void> {
        await runFreeTranslation(req, {
            label: this.serviceName,
            chunk: 4000,
            max: 10000,
            makeChunkTranslator: async () => {
                const target = volcTargetLang(req.toLang)
                return async (chunk) => {
                    const data = await fetchJson(
                        this.serviceName,
                        'https://translate.volcengine.com/crx/translate/v1/',
                        {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ source_language: 'auto', target_language: target, text: chunk }),
                        }
                    )
                    return parseVolcengineText(data)
                }
            },
        })
    }
}
