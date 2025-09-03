import { Configuration, CoreApi } from "@goauthentik/api";

export interface Message {
    version: string;
    path: string;
    profile?: string;
    id: string;
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
        this.#port = chrome.runtime.connectNative("io.goauthentik.agent");
        this.#port.onMessage.addListener(this.#listener.bind(this));
        this.#port.onDisconnect.addListener(() => {
            console.debug("Disconnected, reconnecting");
            this.#connect();
        });
        console.debug("Connected to native");
    }

    #listener(msg: Response) {
        const prom = this.#promises.get(msg.response_to);
        console.debug("Got response", msg);
        if (!prom) {
            console.debug("No promise to resolve");
            return;
        }
        prom.resolve(msg);
    }

    postMessage(msg: Message): Promise<Response> {
        const promise = Promise.withResolvers<Response>();
        this.#promises.set(msg.id, promise);
        console.debug(`Sending message`, msg);
        this.#port?.postMessage(msg);
        return promise.promise;
    }

    async ping(): Promise<Response> {
        const uid = createRandomString();
        return this.postMessage({
            version: "1",
            path: "ping",
            id: uid,
        });
    }

    async getToken(profile: string): Promise<{ token: string; url: string }> {
        const uid = createRandomString();
        const token = await this.postMessage({
            version: "1",
            path: "get_token",
            profile: profile,
            id: uid,
        });
        return {
            token: token.data.token as string,
            url: token.data.url as string,
        };
    }

    async listProfiles(): Promise<{ profiles: { name: string }[] }> {
        const uid = createRandomString();
        const profiles = await this.postMessage({
            version: "1",
            path: "list_profiles",
            id: uid,
        });
        return {
            profiles: profiles.data as unknown as { name: string }[],
        };
    }

    async fetchApplications(profile: string) {
        const token = await this.getToken(profile);

        const response = await new CoreApi(
            new Configuration({
                basePath: `${token.url}/api/v3`,
                accessToken: token.token,
            }),
        ).coreApplicationsList({});
        return response.results;
    }
}
