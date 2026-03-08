import SwiftUI
import Bridge
import Sysd

struct ContentView: View {

    @State private var ping: String = ""
    @State private var facts: String = ""
    @State private var selection = 0

    var body: some View {
        NavigationView {
            TabView(selection: $selection) {
                List {
                    Section(header: Text("Actions")) {
                        Button {
                            Task {
                                try await SysdBridge.shared
                                    .domainsEnroll(
                                        name: "default",
                                        authentikURL: "https://id.beryju.io",
                                        token: "")
                            }
                        } label: {
                            Text("Enroll")
                        }
                    }
                    Section(header: Text("MDM")) {
                        let defaults = UserDefaults.standard
                        let serverConfig = defaults.dictionary(forKey: "com.apple.configuration.managed") as? [String: Any?]
                        serverConfig.map() { key in
                            Text("\(key)")
                        }
                    }
                    Section(header: Text("Facts")) {
                        Text("Ping from sysd \(ping)")
                        Text(facts)
                    }
                }.onAppear() {
                    onAppear()
                }
                .tabItem {
                    Image(systemName: "house")
                    Text("Home")
                }
                .tag(0)
            }
            .toolbar {
                ToolbarItem(placement: .principal) {
                    Image("brand_white")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .padding(.top, 10)
                        .padding(.bottom, 10)
                        .padding(.horizontal, 100)
                }
            }
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(Color(.accent), for: .navigationBar)
            .toolbarBackground(.visible, for: .navigationBar)
        }
    }

    func onAppear() {
        Task {
            do {
                ping =  try await SysdBridge.shared.ping()
            } catch {
                print(error)
            }
        }
        facts = Sysd.MobilebindFacts()
    }
}

#Preview {
    ContentView()
}
