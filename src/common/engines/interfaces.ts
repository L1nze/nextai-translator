export interface IModel {
    id: string
    name: string
    description?: string
}

export interface IMessage {
    role: string
    content: string
}

export interface IMessageRequest {
    rolePrompt: string
    commandPrompt: string
    modelOverride?: string
    thinkingBudget?: number
    // Raw translation payload for non-LLM engines (free machine-translation
    // services read the original text and language codes from these instead
    // of parsing the LLM prompts). LLM engines ignore them.
    sourceText?: string
    fromLang?: string
    toLang?: string
    mode?: string
    onMessage: (message: { content: string; role: string; isFullText?: boolean }) => Promise<void>
    onError: (error: string) => void
    onFinished: (reason: string) => void
    onStatusCode?: (statusCode: number) => void
    signal: AbortSignal
}

export interface IEngine {
    checkLogin: () => Promise<boolean>
    isLocal(): boolean
    supportCustomModel(): boolean
    getModel(): Promise<string>
    listModels(apiKey: string | undefined): Promise<IModel[]>
    sendMessage(req: IMessageRequest): Promise<void>
}
