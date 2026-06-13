import { css, html, LitElement } from "lit";
import { customElement, state } from "lit/decorators.js";
import type { Device, TabId } from "../types.js";
import "./header.js";
import "./tab-bar.js";
import "./device-carousel.js";
import "./device-detail.js";

@customElement("ak-app-shell")
export class AppShell extends LitElement {
    static styles = css`
        :host {
            display: flex;
            flex-direction: column;
            height: 100vh;
            overflow: hidden;
            background: var(--ak-color-surface, #f6f6f6);
        }
        .content {
            flex: 1;
            overflow-y: auto;
        }
    `;

    @state() private _activeTab: TabId = "devices";
    @state() private _selectedDeviceId = "mac-jens";

    private devices: Device[] = [
        {
            id: "mac-jens",
            name: "Jens's MacBook Pro",
            originalName: "Jens's MacBook Pro",
            manufacturer: "Apple",
            status: "compliant",
            isCurrent: true,
        },
        {
            id: "mac-work",
            name: "Work MacBook Air",
            originalName: "Work MacBook Air",
            manufacturer: "Apple",
            status: "compliant",
            badgeCount: 2,
        },
        {
            id: "win-vm",
            name: "Windows Dev VM",
            originalName: "DESKTOP-WIN-VM01",
            manufacturer: "VMware",
            status: "non-compliant",
            badgeCount: 1,
        },
        {
            id: "ipad-pro",
            name: "iPad Pro 12.9",
            originalName: "iPad Pro 12.9",
            manufacturer: "Apple",
            status: "pending",
        },
        {
            id: "linux-server",
            name: "Ubuntu Dev Server",
            originalName: "ubuntu-dev-01",
            manufacturer: "Dell",
            status: "compliant",
        },
    ];

    private get selectedDevice(): Device | null {
        return this.devices.find((d) => d.id === this._selectedDeviceId) ?? null;
    }

    private _onTabChange(e: CustomEvent) {
        this._activeTab = e.detail.tab;
    }

    private _onDeviceSelect(e: CustomEvent) {
        this._selectedDeviceId = e.detail.id;
    }

    render() {
        return html`
            <ak-platform-header></ak-platform-header>
            <ak-tab-bar
                .activeTab=${this._activeTab}
                @ak-tab-change=${this._onTabChange}
            ></ak-tab-bar>
            <ak-device-carousel
                .devices=${this.devices}
                .selectedId=${this._selectedDeviceId}
                @ak-device-select=${this._onDeviceSelect}
            ></ak-device-carousel>
            <div class="content">
                <ak-device-detail .device=${this.selectedDevice}></ak-device-detail>
            </div>
        `;
    }
}
