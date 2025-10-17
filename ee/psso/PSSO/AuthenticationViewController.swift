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
    var cancelFunc: () -> Void = {}

    override init(nibName nibNameOrNil: NSNib.Name?, bundle nibBundleOrNil: Bundle?) {
        super.init(nibName: nibNameOrNil, bundle: nibBundleOrNil)
        self.load()
    }

    required init?(coder: NSCoder) {
        super.init(coder:coder)
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
        self.cancelFunc()
    }

    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
    ) {
        if let url = navigationAction.request.url,
            url.scheme == "io.goauthentik.platform"
        {
            self.logger.debug("Intercepted redirect: \(url.absoluteString)")
            if OIDC.shared.resumeAuthorizationFlow(with: url) {
                decisionHandler(.cancel)
            }
        }
        decisionHandler(.allow)
    }

}
