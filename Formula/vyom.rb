class Vyom < Formula
  desc "A minimalist, transparent music player for the terminal"
  homepage "https://github.com/MrSyr3x/vyom"
  url "https://github.com/MrSyr3x/vyom/archive/refs/tags/v1.0.232.tar.gz"
  version "1.0.232"
  license "MIT"

  depends_on "rust" => :build

  # Runtime dependencies
  depends_on "mpd"
  depends_on "cava" => :recommended
  depends_on "switchaudio-osx" => :recommended

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Simple test to verify version
    assert_match "vyom 1.0.232", shell_output("#{bin}/vyom --version")
  end
end
