import { Configuration, CoreApi } from "@goauthentik/api";
import { type Native } from "./native";

export interface GetToken {
  version: string;
  path: string;
  profile: string;
  id: string;
}

export interface TokenResponse {
  response_to: string;
  token: string;
  url: string;
}

function createRandomString(length: number = 16) {
  const chars =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  let result = "";
  const randomArray = new Uint8Array(length);
  crypto.getRandomValues(randomArray);
  randomArray.forEach((number) => {
    result += chars[number % chars.length];
  });
  return result;
}

export async function getToken(
  n: Native,
): Promise<{ token: string; url: string }> {
  const uid = createRandomString();
  const token = (await n.postMessage({
    version: "1",
    path: "get_token",
    profile: "default",
    id: uid,
  } as GetToken)) as TokenResponse;
  return {
    token: token.token,
    url: token.url,
  };
}

export async function fetchApplications(n: Native) {
  const token = await getToken(n);

  const response = await new CoreApi(
    new Configuration({
      basePath: `${token.url}/api/v3`,
      accessToken: token.token,
    }),
  ).coreApplicationsList({});
  return response.results;
}

export function launchApplication(appUrl: string) {
  chrome.tabs.create({ url: appUrl });
}
