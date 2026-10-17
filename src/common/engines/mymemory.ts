import { AbstractFreeMT, fetchJson, myMemoryTargetLang, runFreeTranslation } from './free-mt'
import { IMessageRequest } from './interfaces'

// MyMemory translation-memory API (api.mymemory.translated.net, no API
// key): anonymous usage is capped around 5000 characters per day and 500
// bytes per request, hence the small chunk size.
export function parseMyMemoryText(data: unknown): string {
    const payload = data as {
        responseDetails?: unknown
        responseStatus?: unknown
        responseData?: { translatedText?: unknown }
    }
    const detail = String(payload?.responseDetails ?? '')
    if (Number(payload?.responseStatus) === 429 || /LIMIT/i.test(detail)) {
        throw new Error(
            'MyMemory free quota is exhausted (~5000 characters per day); retry tomorrow or use another service'
        )
    }
    const out = payload?.responseData?.translatedText
    if (!out || !String(out).trim() || /QUERY LENGTH LIMIT/i.test(String(out))) {
        throw new Error('MyMemory returned no result')
    }
    return String(out)
}

export class MyMemory extends AbstractFreeMT {
    readonly serviceName = 'MyMemory'

    async sendMessage(req: IMessageRequest): Promise<void> {
        await runFreeTranslation(req, {
            label: this.serviceName,
            chunk: 450,
            max: 4000,
            makeChunkTranslator: async () => {
                const target = myMemoryTargetLang(req.toLang)
                return async (chunk) => {
                    const url =
                        'https://api.mymemory.translated.net/get?q=' +
                        encodeURIComponent(chunk) +
                        '&langpair=Autodetect%7C' +
                        encodeURIComponent(target)
                    const data = await fetchJson(this.serviceName, url, { method: 'GET' })
                    return parseMyMemoryText(data)
                }
            },
        })
    }
}
