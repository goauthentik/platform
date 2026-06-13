export interface Device {
    id: string;
    name: string;
    originalName: string;
    manufacturer: string;
    status: "compliant" | "non-compliant" | "pending";
    badgeCount?: number;
    isCurrent?: boolean;
}

export type TabId = "devices" | "apps";
