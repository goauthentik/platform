export interface Message {
    version: string;
    path: string;
    profile?: string;
    id: string;
    data: { [key: string]: unknown };
}

export interface Response {
    response_to: string;
    data: { [key: string]: unknown };
}

function createRandomString(length: number = 16) {
    const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let result = "";
    const randomArray = new Uint8Array(length);
    crypto.getRandomValues(randomArray);
    randomArray.forEach((number) => {
        result += chars[number % chars.length];
    });
    return result;
}

export class Native {
    #port?: chrome.runtime.Port;
    #promises: Map<string, PromiseWithResolvers<Response>> = new Map();

    constructor() {
        this.#connect();
    }

    #connect() {
        this.#port = chrome.runtime.connectNative("io.goauthentik.platform");
        this.#port.onMessage.addListener(this.#listener.bind(this));
        this.#port.onDisconnect.addListener(() => {
            console.debug("authentik/bext/native: Disconnected, reconnecting");
            this.#connect();
        });
        console.debug("authentik/bext/native: Connected to native");
    }

    #listener(msg: Response) {
        const prom = this.#promises.get(msg.response_to);
        console.debug(`authentik/bext/native[${msg.response_to}]: Got response`);
        if (!prom) {
            console.debug(`authentik/bext/native[${msg.response_to}]: No promise to resolve`);
            return;
        }
        prom.resolve(msg);
    }

    postMessage(msg: Partial<Message>): Promise<Response> {
        msg.id = createRandomString();
        const promise = Promise.withResolvers<Response>();
        this.#promises.set(msg.id, promise);
        console.debug(`authentik/bext/native[${msg.id}]: Sending message ${msg.path}`);
        this.#port?.postMessage(msg);
        return promise.promise;
    }

    async ping(): Promise<Response> {
        return this.postMessage({
            version: "1",
            path: "ping",
        });
    }

    async getToken(profile: string): Promise<{ token: string; url: string }> {
        const token = await this.postMessage({
            version: "1",
            path: "get_token",
            profile: profile,
        });
        return {
            token: token.data.token as string,
            url: token.data.url as string,
        };
    }

    async listProfiles(): Promise<{ profiles: { name: string }[] }> {
        const profiles = await this.postMessage({
            version: "1",
            path: "list_profiles",
        });
        return {
            profiles: profiles.data.profiles as unknown as { name: string }[],
        };
    }

    async platformSignEndpointHeader(profile: string, challenge: string): Promise<string> {
        const response = await this.postMessage({
            version: "1",
            path: "platform_sign_endpoint_header",
            profile: profile,
            data: {
                challenge:challenge
            }
        });
        return response.data.response as string;
    }
}
