package managedconfig

import (
	"bytes"
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/platform/pstr"
)

const mockOutput = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>_computerlevel</key>
	<array>
		<dict>
			<key>ProfileDisplayName</key>
			<string>authentik Platform</string>
			<key>ProfileIdentifier</key>
			<string>AED6446C-10F2-4D4D-94A9-119B23D85E0C</string>
			<key>ProfileInstallDate</key>
			<string>2025-10-17 21:36:52 +0000</string>
			<key>ProfileItems</key>
			<array>
				<dict>
					<key>PayloadContent</key>
					<dict>
						<key>RegistrationToken</key>
						<string>insecure-placeholder-for-testing</string>
						<key>URL</key>
						<string>https://authentik.company</string>
					</dict>
					<key>PayloadDisplayName</key>
					<string>authentik Platform</string>
					<key>PayloadIdentifier</key>
					<string>io.goauthentik.platform.5B7CF082-647E-4D29-B008-BA2D62C0F74F</string>
					<key>PayloadType</key>
					<string>io.goauthentik.platform</string>
					<key>PayloadUUID</key>
					<string>5B7CF082-647E-4D29-B008-BA2D62C0F74F</string>
					<key>PayloadVersion</key>
					<integer>1</integer>
				</dict>
			</array>
			<key>ProfileType</key>
			<string>Configuration</string>
			<key>ProfileUUID</key>
			<string>AED6446C-10F2-4D4D-94A9-119B23D85E0C</string>
			<key>ProfileVerificationState</key>
			<string>verified</string>
			<key>ProfileVersion</key>
			<integer>1</integer>
		</dict>
	</array>
</dict>
</plist>
`

func TestDarwin(t *testing.T) {
	type Config struct {
		RegistrationToken string
		URL               string
	}

	execProfileCmd = func() (*bytes.Buffer, error) {
		b := &bytes.Buffer{}
		b.WriteString(mockOutput)
		return b, nil
	}

	p, err := Get[Config](pstr.PlatformString{
		Darwin: pstr.S("io.goauthentik.platform"),
	})
	assert.NoError(t, err)
	assert.Equal(t, &Config{
		RegistrationToken: "insecure-placeholder-for-testing",
		URL:               "https://authentik.company",
	}, p)
}
