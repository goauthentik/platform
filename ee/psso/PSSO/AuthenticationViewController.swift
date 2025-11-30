import AuthenticationServices
import Cocoa
import OSLog
import Sentry
import WebKit

class AuthenticationViewController: NSViewController, WKNavigationDelegate {
    @IBOutlet var webView: WKWebView!
    @IBOutlet var cancelButton: NSButton!

    var logger: Logger = Logger(
        subsystem: Bundle.main.bundleIdentifier!, category: "AuthenticationViewController")

    var authorizationRequest: URL?
    var interactive: InteractiveAuth?

    override init(nibName nibNameOrNil: NSNib.Name?, bundle nibBundleOrNil: Bundle?) {
        super.init(nibName: nibNameOrNil, bundle: nibBundleOrNil)
        self.load()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        self.load()
    }

    deinit {
        self.unload()
    }

    private func load() {
        self.logger.debug("Load")
        Sentry.setup()
    }

    private func unload() {
        self.logger.debug("Unload")
        Sentry.flush()
    }

    override func viewDidAppear() {
        let appVersion = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String
        let systemVersion = ProcessInfo.processInfo.operatingSystemVersionString
        self.webView.customUserAgent =
            "authentik Platform/PSSO@\(appVersion ?? "dev") (OS \(systemVersion))"
        self.webView.navigationDelegate = self
        self.webView.isInspectable = true
        if let url = authorizationRequest {
            self.logger.debug("navigating to URL")
            webView.load(URLRequest(url: url))
        }
    }

    override var nibName: NSNib.Name? {
        return NSNib.Name("AuthenticationViewController")
    }

    @IBAction func clickCancel(_: Any) {
        self.interactive?.cancelAuth()
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction) async
        -> WKNavigationActionPolicy
    {
        self.logger.debug("Navigate \(String(describing: navigationAction.request.url))")
        guard let interactive = self.interactive else {
            return .allow
        }
        if let url = navigationAction.request.url,
            interactive.isFinishedURL(url: url)
        {
            self.logger.debug("Intercepted redirect: \(url.absoluteString)")
            if await interactive.resumeAuthorizationFlow(with: url) {
                return .cancel
            }
        }
        return await interactive.injectDTH(
            webView,
            decidePolicyFor: navigationAction,
        )
    }

}
