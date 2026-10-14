import SwiftUI

// MARK: - Model

struct AccessRequestModel {
    var title: String = "authentik Access Requested"
    var requestingApp: String = ""
    var appIcon: Image? = nil
    var badgeIcon: Image? = nil
    var accentColor: Color = Color(hex: 0xFD4B2D)

    var profileName: String = ""
    var profileEmail: String = ""
    var profileUsername: String = ""
    var profileGroups: String = ""
    var profileAvatar: Image? = nil

    var reason: String = ""

    var initials: String {
        let parts = profileName.split(separator: " ").prefix(2)
        let s = parts.compactMap { $0.first }.map(String.init).joined().uppercased()
        return s.isEmpty ? "U" : s
    }

    var hasProfileData: Bool {
        !profileName.isEmpty || !profileEmail.isEmpty || !profileUsername.isEmpty || profileAvatar != nil
    }
}

// MARK: - View

struct AuthentikAccessRequestView: View {
    var request = AccessRequestModel()
    var authz: LocalAuthenticationView?
    var onDeny: () -> Void = {}

    @State private var approveAll = false
    @State private var expanded = false

    private let cardBackground = Color(light: 0xF4F2EC, dark: 0x2A2928)
    private let surface = Color(light: 0xFFFFFF, dark: 0x363533)
    private let primaryText = Color(light: 0x1C1B1A, dark: 0xF2F0EB)
    private let bodyText = Color(light: 0x2A2826, dark: 0xDDDAD3)
    private let mutedText = Color(light: 0x8A8780, dark: 0x9A978F)
    private let lineColor = Color(light: 0xCFCDC6, dark: 0x4A4844)

    var body: some View {
        VStack(spacing: 0) {
            Text(request.title)
                .font(.system(size: 22, weight: .bold))
                .foregroundStyle(primaryText)
                .padding(.bottom, 22)

            iconRow
                .padding(.bottom, 22)

            subtitle
                .padding(.bottom, 18)

            if request.hasProfileData {
                profileCard

                if expanded {
                    profileDetails
                        .padding(.top, 10)
                }
            }

            footer
        }
        .padding(EdgeInsets(top: 30, leading: 34, bottom: 26, trailing: 34))
        .frame(width: 460)
        .background(
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .fill(cardBackground)
                .overlay(
                    RoundedRectangle(cornerRadius: 18, style: .continuous)
                        .strokeBorder(Color.primary.opacity(0.18), lineWidth: 0.5)
                )
        )
        .shadow(color: .black.opacity(0.30), radius: 25, x: 0, y: 22)
        .animation(.easeInOut(duration: 0.15), value: expanded)
    }

    // MARK: Icon row

    private var iconRow: some View {
        HStack(spacing: 0) {
            applicationIcon
                .frame(width: 64, height: 74)

            connector
                .frame(width: 92)

            authentikBadge
        }
    }

    @ViewBuilder
    private var applicationIcon: some View {
        if let icon = request.appIcon {
            icon
                .resizable()
                .aspectRatio(contentMode: .fill)
                .frame(width: 62, height: 62)
                .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
                .shadow(color: .black.opacity(0.22), radius: 2.5, x: 0, y: 2)
        } else {
            ZStack(alignment: .topTrailing) {
                RoundedRectangle(cornerRadius: 7, style: .continuous)
                    .fill(surface)
                    .overlay(
                        RoundedRectangle(cornerRadius: 7, style: .continuous)
                            .strokeBorder(Color.primary.opacity(0.16), lineWidth: 0.5)
                    )
                    .frame(width: 60, height: 74)
                    .shadow(color: .black.opacity(0.12), radius: 1, x: 0, y: 1)
                // folded corner
                Triangle()
                    .fill(Color(light: 0xE7E4DC, dark: 0x4A4844))
                    .frame(width: 16, height: 16)
            }
            .frame(width: 60, height: 74)
        }
    }

    private var connector: some View {
        HStack(spacing: 4) {
            Rectangle().fill(lineColor).frame(height: 2)
            ZStack {
                Circle().fill(Color(hex: 0x34C759))
                Image(systemName: "checkmark")
                    .font(.system(size: 9, weight: .bold))
                    .foregroundStyle(.white)
            }
            .frame(width: 22, height: 22)
            .shadow(color: .black.opacity(0.18), radius: 1, x: 0, y: 1)
            Rectangle().fill(lineColor).frame(height: 2)
        }
    }

    private var authentikBadge: some View {
        Group {
            if let badge = request.badgeIcon {
                badge
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(width: 64, height: 64)
            }
        }
    }

    // MARK: Subtitle

    private var subtitle: some View {
        Text(self.request.reason)
        .font(.system(size: 14))
        .foregroundStyle(bodyText)
        .multilineTextAlignment(.center)
    }

    // MARK: Profile card

    private var profileCard: some View {
        Button {
            expanded.toggle()
        } label: {
            HStack(spacing: 12) {
                Image(systemName: "chevron.right")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(mutedText)
                    .rotationEffect(.degrees(expanded ? 90 : 0))

                avatar

                VStack(alignment: .leading, spacing: 1) {
                    Text(request.profileName)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(primaryText)
                        .lineLimit(1)
                    Text(request.profileEmail)
                        .font(.system(size: 12))
                        .foregroundStyle(mutedText)
                        .lineLimit(1)
                }
                Spacer(minLength: 0)
            }
            .padding(.vertical, 11)
            .padding(.horizontal, 15)
            .background(
                RoundedRectangle(cornerRadius: 11, style: .continuous)
                    .fill(surface)
                    .overlay(
                        RoundedRectangle(cornerRadius: 11, style: .continuous)
                            .strokeBorder(Color.primary.opacity(0.10), lineWidth: 0.5)
                    )
                    .shadow(color: .black.opacity(0.05), radius: 1, x: 0, y: 1)
            )
        }
        .buttonStyle(.plain)
    }

    @ViewBuilder
    private var avatar: some View {
        if let img = request.profileAvatar {
            img
                .resizable()
                .aspectRatio(contentMode: .fill)
                .frame(width: 38, height: 38)
                .clipShape(Circle())
        } else {
            ZStack {
                Circle().fill(
                    RadialGradient(
                        colors: [Color(hex: 0xFF7A4D), request.accentColor],
                        center: UnitPoint(x: 0.30, y: 0.25),
                        startRadius: 0, endRadius: 38
                    )
                )
                Text(request.initials)
                    .font(.system(size: 14, weight: .bold))
                    .foregroundStyle(.white)
            }
            .frame(width: 38, height: 38)
        }
    }

    private var profileDetails: some View {
        VStack(spacing: 8) {
            detailRow("Username", request.profileUsername)
            detailRow("Groups", request.profileGroups)
        }
        .padding(.vertical, 12)
        .padding(.horizontal, 15)
        .background(
            RoundedRectangle(cornerRadius: 9, style: .continuous)
                .fill(Color(light: 0xFAF9F5, dark: 0x31302E))
                .overlay(
                    RoundedRectangle(cornerRadius: 9, style: .continuous)
                        .strokeBorder(Color.primary.opacity(0.08), lineWidth: 0.5)
                )
        )
    }

    private func detailRow(_ label: String, _ value: String) -> some View {
        HStack {
            Text(label)
                .foregroundStyle(mutedText)
            Spacer()
            Text(value)
                .fontWeight(.semibold)
                .foregroundStyle(bodyText)
        }
        .font(.system(size: 12.5))
    }

    // MARK: Footer

    private var footer: some View {
        HStack {
            Button("Deny", action: onDeny)
                .buttonStyle(.bordered)
                .controlSize(.large)
                .keyboardShortcut(.escape, modifiers: [])

            Spacer()

            authz
                .frame(width: LocalAuthenticationView.size, height: LocalAuthenticationView.size)
        }
        .padding(.top, 18)
    }
}

// MARK: - Helpers

private struct Triangle: Shape {
    func path(in rect: CGRect) -> Path {
        var p = Path()
        p.move(to: CGPoint(x: rect.minX, y: rect.minY))
        p.addLine(to: CGPoint(x: rect.maxX, y: rect.minY))
        p.addLine(to: CGPoint(x: rect.maxX, y: rect.maxY))
        p.closeSubpath()
        return p
    }
}

extension Color {
    init(hex: UInt, alpha: Double = 1) {
        self.init(
            .sRGB,
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255,
            opacity: alpha
        )
    }
}

extension Color {
    /// Resolves per appearance, so the view follows the system light/dark setting.
    init(light: UInt, dark: UInt) {
        self.init(nsColor: NSColor(name: nil) { appearance in
            let hex = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? dark : light
            return NSColor(Color(hex: hex))
        })
    }
}

// MARK: - Preview

#Preview {
    AuthentikAccessRequestView(
        request: AccessRequestModel(
            title: "authentik Access Requested",
            requestingApp: "iTermServer-3.6.11",
            profileName: "Jane Doe",
            profileEmail: "jane@goauthentik.io",
            profileUsername: "jdoe",
            profileGroups: "Engineering, Admins"
        ),
        authz: nil
    )
    .padding(48)
    .background(
        LinearGradient(
            colors: [Color(hex: 0xC9C7CC), Color(hex: 0xB7B4BB)],
            startPoint: .topLeading, endPoint: .bottomTrailing
        )
    )
}
