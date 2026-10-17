/* eslint-disable camelcase */
import { createCipheriv } from 'node:crypto'
import { describe, expect, it } from 'vitest'
import {
    aes128EcbEncryptPkcs7,
    bytesToBase64,
    googleTargetLang,
    googleTk,
    myMemoryTargetLang,
    runFreeTranslation,
    splitChunks,
    toUrlSafeBase64,
    transmartTargetLang,
    utf8Bytes,
    cnkiTargetLang,
} from './free-mt'
import { buildCnkiWords, parseCnkiText, parseCnkiToken } from './cnki'
import { parseGoogleFreeText } from './google-translate'
import { parseMyMemoryText } from './mymemory'
import { parseTransmartText } from './transmart'
import { parseVolcengineText } from './volcengine'

function nodeAesEcbBase64(text: string, key: string): string {
    const cipher = createCipheriv('aes-128-ecb', Buffer.from(key, 'utf8'), null)
    return cipher.update(text, 'utf8', 'base64') + cipher.final('base64')
}

function hexToBytes(hex: string): Uint8Array {
    const out = new Uint8Array(hex.length / 2)
    for (let i = 0; i < out.length; i++) {
        out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16)
    }
    return out
}

function bytesToHex(bytes: Uint8Array): string {
    return Array.from(bytes)
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('')
}

describe('AES-128-ECB (free-mt)', () => {
    it('matches the FIPS-197 appendix C.1 vector', () => {
        const key = hexToBytes('000102030405060708090a0b0c0d0e0f')
        const plain = hexToBytes('00112233445566778899aabbccddeeff')
        const out = aes128EcbEncryptPkcs7(plain, key)
        // PKCS#7 adds a full padding block, so the first 16 bytes are the
        // vector ciphertext.
        expect(bytesToHex(out.slice(0, 16))).toBe('69c4e0d86a7b0430d8cdb78070b4c55a')
        expect(out.length).toBe(32)
    })

    it.each([
        ['short', 'hello world'],
        ['chinese', '你好，世界！这是一段中文文本。'],
        ['exact block', 'abcdefghijklmnop'],
        ['multi block', 'The quick brown fox jumps over the lazy dog. 敏捷的棕色狐狸跳过了懒狗。'.repeat(3)],
    ])('matches Node crypto aes-128-ecb with PKCS#7 (%s)', (_name, text) => {
        const key = '4e87183cfd3a45fe'
        const mine = bytesToBase64(aes128EcbEncryptPkcs7(utf8Bytes(text), utf8Bytes(key)))
        expect(mine).toBe(nodeAesEcbBase64(text, key))
    })
})

describe('splitChunks (free-mt)', () => {
    it('does not split short text', () => {
        expect(splitChunks('Hello world', 100)).toEqual(['Hello world'])
        expect(splitChunks('', 100)).toEqual([''])
    })

    it('splits at sentence boundaries and re-joins to the original', () => {
        const text = 'Pi is 3.14 values. Dr. Smith lives nearby. It costs $3 only. That is nice!'
        const chunks = splitChunks(text, 25)
        expect(chunks.join('')).toBe(text)
        chunks.forEach((c) => expect(c.length).toBeLessThanOrEqual(25))
        expect(chunks.some((c) => c.includes('3.14 values.'))).toBe(true)
    })

    it('hard-splits long unpunctuated text', () => {
        expect(splitChunks('a'.repeat(25), 10)).toEqual(['a'.repeat(10), 'a'.repeat(10), 'a'.repeat(5)])
    })

    it('keeps closing quotes with the preceding sentence', () => {
        const chunks = splitChunks('他说：“你好。”然后离开。', 8)
        expect(chunks.join('')).toBe('他说：“你好。”然后离开。')
        expect(chunks[0]).toContain('。”')
    })
})

describe('Google tk signature (free-mt)', () => {
    it('is stable and shaped like remainder.xor', () => {
        const tk = googleTk('Attention is all you need')
        expect(tk).toMatch(/^\d+\.\d+$/)
        expect(googleTk('Attention is all you need')).toBe(tk)
    })

    it('handles non-latin input without throwing', () => {
        expect(googleTk('你好世界🌏')).toMatch(/^\d+\.\d+$/)
    })
})

describe('target-language mapping (free-mt)', () => {
    it('maps Chinese variants per service', () => {
        expect(googleTargetLang('zh-Hans')).toBe('zh-CN')
        expect(googleTargetLang('zh-Hant')).toBe('zh-TW')
        expect(googleTargetLang('en-US')).toBe('en')
        expect(transmartTargetLang('zh-Hans')).toBe('zh')
        expect(myMemoryTargetLang('yue')).toBe('zh-CN')
    })

    it('throws for unsupported target languages', () => {
        expect(() => cnkiTargetLang('ja')).toThrow('Chinese <-> English')
        expect(() => transmartTargetLang('ceb')).toThrow('does not support')
    })
})

describe('response parsers (free engines)', () => {
    it('parses Google segment arrays', () => {
        expect(parseGoogleFreeText([[['你好', 'hello', null, null], null, ['世界', 'world', null, null]]])).toBe(
            '你好世界'
        )
        expect(() => parseGoogleFreeText({})).toThrow('unparsable')
        expect(() => parseGoogleFreeText([[[]]])).toThrow('no result')
    })

    it('parses Transmart auto_translation', () => {
        expect(parseTransmartText({ auto_translation: ['你好', null, '世界'] })).toBe('你好\n世界')
        expect(() => parseTransmartText({})).toThrow('unparsable')
    })

    it('parses Volcengine translation', () => {
        expect(parseVolcengineText({ translation: '你好' })).toBe('你好')
        expect(() => parseVolcengineText({ translation: '' })).toThrow('no result')
    })

    it('parses CNKI token and payload', () => {
        expect(parseCnkiToken({ code: 200, data: 'tok' })).toBe('tok')
        expect(() => parseCnkiToken({ code: 500 })).toThrow('token')
        expect(parseCnkiText({ data: { mResult: '你好' } })).toBe('你好')
        expect(() => parseCnkiText({ data: { isInputVerificationCode: true } })).toThrow('rate limiting')
        expect(() => parseCnkiText({ data: {} })).toThrow('no result')
    })

    it('parses MyMemory and surfaces quota errors', () => {
        expect(parseMyMemoryText({ responseStatus: 200, responseData: { translatedText: '你好' } })).toBe('你好')
        expect(() => parseMyMemoryText({ responseStatus: 429 })).toThrow('quota')
        expect(() =>
            parseMyMemoryText({ responseStatus: 200, responseData: { translatedText: 'QUERY LENGTH LIMIT' } })
        ).toThrow()
    })
})

describe('runFreeTranslation orchestration', () => {
    function makeReq(overrides: Partial<Parameters<typeof runFreeTranslation>[0]> = {}) {
        const messages: string[] = []
        const events: string[] = []
        const req = {
            rolePrompt: '',
            commandPrompt: '',
            sourceText: 'Hello world. Nice day.',
            toLang: 'zh-Hans',
            mode: 'translate',
            signal: new AbortController().signal,
            onMessage: async (m: { content: string }) => {
                messages.push(m.content)
            },
            onError: (e: string) => events.push('error:' + e),
            onFinished: (r: string) => events.push('finished:' + r),
            ...overrides,
        }
        return { req, messages, events }
    }

    it('emits each chunk and finishes with stop', async () => {
        const { req, messages, events } = makeReq()
        await runFreeTranslation(req, {
            label: 'Test',
            chunk: 12,
            max: 100,
            makeChunkTranslator: async () => async (chunk) => `《${chunk}》`,
        })
        expect(messages.length).toBeGreaterThan(1)
        expect(messages.join('')).toContain('《Hello world.》')
        expect(events).toEqual(['finished:stop'])
    })

    it('rejects non-translate modes', async () => {
        const { req, events } = makeReq({ mode: 'polishing' })
        await runFreeTranslation(req, {
            label: 'Test',
            chunk: 100,
            max: 100,
            makeChunkTranslator: async () => async () => '',
        })
        expect(events[0]).toContain('only supports translation')
        expect(events).toContain('finished:error')
    })

    it('rejects texts over the service maximum', async () => {
        const { req, events } = makeReq({ sourceText: 'a'.repeat(11) })
        await runFreeTranslation(req, {
            label: 'Test',
            chunk: 100,
            max: 10,
            makeChunkTranslator: async () => async () => '',
        })
        expect(events[0]).toContain('cannot translate more than 10')
    })

    it('surfaces unsupported target language via onError', async () => {
        const { req, events } = makeReq({ toLang: 'ja' })
        await runFreeTranslation(req, {
            label: 'Test',
            chunk: 100,
            max: 100,
            makeChunkTranslator: async () => {
                cnkiTargetLang('ja') // throws before any network access
                return async () => ''
            },
        })
        expect(events[0]).toContain('Chinese <-> English')
    })

    it('stops quietly once the request is aborted', async () => {
        const controller = new AbortController()
        controller.abort()
        const { req, events } = makeReq({ signal: controller.signal })
        await runFreeTranslation(req, {
            label: 'Test',
            chunk: 5,
            max: 100,
            makeChunkTranslator: async () => async () => 'x',
        })
        expect(events).toEqual(['finished:stop'])
    })
})

describe('CNKI words payload (free-mt + cnki)', () => {
    it('is URL-safe base64 of AES-ECB ciphertext matching Node crypto', () => {
        const text = '机器学习在自然语言处理中的应用'
        const expected = toUrlSafeBase64(nodeAesEcbBase64(text, '4e87183cfd3a45fe'))
        expect(buildCnkiWords(text)).toBe(expected)
    })
})
