cask "wick" do
  version "0.3.0"
  sha256 "d7e2bb5e5781f036abfdbbe1093e61d5247af3c8dd3b852ec0418531ba83abc0"

  url "https://github.com/Abulkosim/wick/releases/download/v#{version}/Wick_#{version}_universal.dmg"
  name "Wick"
  desc "Menu bar app that prevents sleep"
  homepage "https://abulkosim.github.io/wick/"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: :monterey

  app "Wick.app"
  binary "#{appdir}/Wick.app/Contents/MacOS/wick"

  uninstall quit: "com.abulkosim.wick"

  zap trash: [
    "~/Library/Application Support/com.abulkosim.wick",
    "~/Library/Caches/com.abulkosim.wick",
    "~/Library/LaunchAgents/Wick.plist",
    "~/Library/WebKit/com.abulkosim.wick",
  ]

  caveats <<~EOS
    Wick is not notarized, so macOS blocks the first launch.
    Open System Settings → Privacy & Security, click Open Anyway, then Open.
    On macOS 12–14, Control-click Wick in Applications → Open also works.
    The wick command (wick status, wick start 45) runs once Wick has opened.
  EOS
end
