import { v4 as uuidv4 } from 'uuid'
import { getUniversalFetch } from '../universal-fetch'
import { getSettings } from '../utils'
import { AbstractOpenAI } from './abstract-openai'
import { IModel } from './interfaces'

// OpenCode Go (https://opencode.ai) is a subscription gateway that fronts
// coding-oriented models (GLM, Kimi, DeepSeek, MiMo, ...) with an
// OpenAI-compatible endpoint:
// https://opencode.ai/inference/go/openai/v1/chat/completions
// The gateway asks clients to identify themselves with a custom user agent
// and a stable x-opencode-session header so abuse can be monitored. The
// session ID is generated once per runtime context (background service
// worker / app run). In browser contexts the User-Agent header is forbidden
// and silently dropped; in the Tauri desktop app it is sent as-is.
const OPENCODE_GO_SESSION_ID = uuidv4()

const OPENCODE_CLIENT_HEADERS: Record<string, string> = {
    'User-Agent': 'NextAITranslator/0.1.0',
    'x-opencode-session': OPENCODE_GO_SESSION_ID,
}

export class OpenCode extends AbstractOpenAI {
    async listModels(apiKey_: string | undefined): Promise<IModel[]> {
        let apiKey = apiKey_
        if (!apiKey) {
            apiKey = await this.getAPIKey()
        }
        if (!apiKey) {
            return []
        }
        // The Go models endpoint is not nested under /openai/v1 like the
        // chat endpoint, so it cannot be derived from getAPIURL().
        const url = 'https://opencode.ai/inference/go/v1/models'
        const fetcher = getUniversalFetch()
        const response = await fetcher(url, {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${apiKey}`,
                ...OPENCODE_CLIENT_HEADERS,
            },
        })
        if (response.status !== 200) {
            if (response.status === 401 || response.status === 403) {
                throw new Error('Invalid API key')
            }
            if (response.status === 404) {
                throw new Error('Invalid API URL')
            }
            throw new Error(`Failed to list models: ${response.statusText}`)
        }
        const json = await response.json()
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        return (json.data ?? []).map((model: any) => {
            return {
                id: model.id,
                name: model.id,
            }
        })
    }

    async getAPIModel(): Promise<string> {
        const settings = await getSettings()
        return settings.openCodeAPIModel
    }
    async getAPIKey(): Promise<string> {
        const settings = await getSettings()
        return settings.openCodeAPIKey
    }
    async getAPIURL(): Promise<string> {
        return 'https://opencode.ai/inference/go/openai/v1'
    }
    async getAPIURLPath(): Promise<string> {
        return '/chat/completions'
    }
    async getHeaders(): Promise<Record<string, string>> {
        return {
            ...(await super.getHeaders()),
            ...OPENCODE_CLIENT_HEADERS,
        }
    }
}
