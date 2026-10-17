import { urlJoin } from 'url-join-ts'
import { getUniversalFetch } from '../universal-fetch'
import { getSettings } from '../utils'
import { AbstractOpenAI } from './abstract-openai'
import { IModel } from './interfaces'

// Alibaba DashScope (通义千问) exposes Qwen models through an
// OpenAI-compatible mode: https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions
export class DashScope extends AbstractOpenAI {
    async listModels(apiKey_: string | undefined): Promise<IModel[]> {
        let apiKey = apiKey_
        if (!apiKey) {
            apiKey = await this.getAPIKey()
        }
        const apiURL = await this.getAPIURL()
        if (!apiKey || !apiURL) {
            return []
        }
        const url = urlJoin(apiURL, '/v1/models')
        const fetcher = getUniversalFetch()
        const response = await fetcher(url, {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${apiKey}`,
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
        return json.data.map((model: any) => {
            return {
                id: model.id,
                name: model.id,
            }
        })
    }

    async getAPIModel(): Promise<string> {
        const settings = await getSettings()
        return settings.dashScopeAPIModel
    }
    async getAPIKey(): Promise<string> {
        const settings = await getSettings()
        return settings.dashScopeAPIKey
    }
    async getAPIURL(): Promise<string> {
        return 'https://dashscope.aliyuncs.com/compatible-mode'
    }
    async getAPIURLPath(): Promise<string> {
        return '/v1/chat/completions'
    }
}
