# Opt-Outta

**Take back your personal data.** Opt-Outta is a free, open source desktop app that automates data removal requests to data brokers. Your personal information never leaves your computer.

![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)

## How It Works

Data brokers collect and sell your personal information — your name, address, phone number, email, and more. Removing yourself means submitting opt-out forms to dozens of sites, each with their own process. Opt-Outta automates this.

1. **Enter your info once** — stored encrypted on your machine, never sent to any server
2. **Pick a broker and run its playbook** — community-created step-by-step automation scripts
3. **The app follows the playbook** — navigating opt-out pages, filling forms, and submitting requests
4. **You handle the human parts** — CAPTCHAs, email confirmations, identity verification
5. **Track your progress** — see which brokers you've opted out of and when to re-check

### What Are Playbooks?

Playbooks are community-submitted automation scripts that describe how to opt out of a specific broker. Each playbook is a sequence of steps — navigate to a URL, fill a field, click a button, wait for a page load, etc. Playbooks are reviewed and cryptographically signed by the Opt-Outta team before the app will run them.

### Privacy Architecture

Your personal data **never** touches our servers. Here's how:

- Playbooks define steps like: `"Fill field with selector '#email' using profileKey: email"`
- The app executes the step using your actual email from local encrypted storage
- The playbook never contains your PII — only selectors and profile key references

Your PII stays on your machine at all times.

---

## Getting Started

### Prerequisites

- **Operating system:** macOS, Windows, or Linux
- **Chrome or Chromium:** Needed for browser automation (the app will detect your installation)

### Install from Download

1. Go to the [Releases](https://github.com/daringdesigns/opt-outta-app/releases) page
2. Download the installer for your platform:
   - **macOS:** `Opt-Outta_x.x.x_aarch64.dmg` (Apple Silicon) or `Opt-Outta_x.x.x_x64.dmg` (Intel)
   - **Windows:** `Opt-Outta_x.x.x_x64-setup.exe`
   - **Linux:** `opt-outta_x.x.x_amd64.AppImage` or `.deb`
3. Install and open the app
4. Follow the setup wizard

### Build from Source

If you'd rather build it yourself (we respect that):

```bash
# Clone the repo
git clone https://github.com/daringdesigns/opt-outta-app.git
cd opt-outta-app

# Install frontend dependencies
npm install

# Install Rust (if you don't have it)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Tauri CLI
cargo install tauri-cli

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

#### System Dependencies

**macOS:**
```bash
xcode-select --install
```

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**Windows:**
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload
- Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (usually pre-installed on Windows 10/11)

---

## First Run

### 1. Set Up Your Profile

Go to **Profile** and fill in the information you want to use for opt-out requests:

- **Required:** First name, last name, email address
- **Recommended:** Phone number, home address, date of birth
- **Optional:** Previous addresses, alternate emails/phones (helps match records on broker sites)

Your profile is encrypted at rest using a key derived from your OS keychain.

### 2. Browse Brokers

Head to **Brokers** to see the full list of supported data brokers. Each one shows:

- **Category** — people-search, marketing, background-check, data-aggregator
- **Method** — web form, email, or API
- **Difficulty** — easy, medium, hard (based on how many steps and verifications are needed)
- **Your status** — not started, submitted, pending verification, confirmed, re-listed

### 3. Run Your First Opt-Out

Pick a broker, select a playbook, and click **Start Opt-Out Run**.

**What to expect:**

- A browser window will open and you'll see the app following the playbook steps
- The app navigates to the opt-out page, fills in your profile data, and submits the form
- When a CAPTCHA appears, the app pauses and asks you to solve it
- When email verification is needed, you'll get a prompt to check your inbox and click the link
- After each broker, the result is logged and the app moves to the next one

A typical run through 15-20 brokers takes about 20-30 minutes, mostly waiting on you for CAPTCHAs and verification emails.

---

## How Playbooks Work

For each broker, the app follows the playbook's steps:

```
1. Navigate to the broker's opt-out page
2. Fill form fields using your local profile data
3. Select options from dropdowns
4. Check required checkboxes
5. Click submit
6. Wait for confirmation
```

Each step specifies an action (`navigate`, `fill`, `select`, `check`, `click`, `wait`, `wait_for`, `captcha`, `scroll_to`, `find_and_click`), a CSS selector, and optionally a profile key or value.

Playbooks are community-created and reviewed by the Opt-Outta team. When a broker changes their opt-out form, the community submits updated playbooks — no app update needed.

---

## Broker Registry

The broker registry lives in this repo as a single JSON file. This is the source of truth — the app bundles it at build time, and [opt-outta.com](https://opt-outta.com) serves it via API for update checks.

### Registry File

```
registry/brokers.json
```

### Format

```json
{
  "version": "2025.02.15.1",
  "brokers": [
    {
      "id": "spokeo",
      "name": "Spokeo",
      "url": "https://www.spokeo.com",
      "category": "people-search",
      "method": "web-form",
      "opt_out_url": "https://www.spokeo.com/optout",
      "known_fields": [
        { "label": "Profile URL", "type": "text", "profile_key": null },
        { "label": "Email", "type": "email", "profile_key": "email" }
      ],
      "notes": "Requires you to find your profile URL first via search, then paste it into the opt-out form.",
      "requires_verification": "email",
      "relist_days": 90,
      "difficulty": "medium",
      "last_verified": "2025-02-01"
    }
  ]
}
```

### Currently Supported Brokers

Browse the full list in the app under **Brokers**, or open `registry/brokers.json` directly.

Categories include:
- **People-search** — Spokeo, BeenVerified, WhitePages, TruePeopleSearch, etc.
- **Marketing** — data aggregators that sell contact lists
- **Background-check** — Intelius, PeopleFinder, USSearch, etc.
- **Data aggregators** — Radaris, Nuwber, ThatsThem, etc.

### Adding a Broker

Edit `registry/brokers.json`, add your entry following the format above, and submit a PR. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full field reference and guidelines.

You can also request a broker without doing the research yourself:

1. **In the app** — click "Request New Broker" on the Brokers page
2. **On GitHub** — [open an issue](https://github.com/daringdesigns/opt-outta-app/issues/new?template=broker-request.md) using the broker request template

### Updating a Broker

If you've recently gone through a broker's opt-out flow, submit a PR updating the `last_verified` date and noting any field changes. This helps everyone.

### How Updates Reach the App

The app ships with a bundled copy of `brokers.json`. On launch (and periodically), it checks opt-outta.com for a newer version number. If one exists, it pulls the latest `brokers.json` from this repo's GitHub raw URL and caches it locally. No account needed.

---

## Re-Listing

Many data brokers will re-add your information after 30-90 days. The app tracks known re-listing timelines for each broker and will alert you on the Dashboard when it's time to re-run the playbook.

---

## Data Storage

Everything is stored locally on your machine:

| Data | Location | Encrypted |
|------|----------|-----------|
| Profile (PII) | OS-specific app data directory | Yes (OS keychain key) |
| Submission history | OS-specific app data directory | No (contains no PII — only broker IDs, dates, statuses) |
| Broker registry cache | OS-specific app data directory | No |

### Exporting Your Data

- **Profile:** Settings → Export Profile (password-protected encrypted file)
- **History:** History → Export (CSV or JSON)

### Deleting Everything

Settings → Delete All Data. This securely wipes your profile, history, and cached registry.

---

## FAQ

### Does this cost money?

The app is completely free and open source.

### Is my data really private?

Yes. Your PII is encrypted on your machine and never sent anywhere except directly to the data broker's opt-out form via the browser. Playbooks only contain page selectors and profile key references — never your actual data. The app is open source so you can verify this yourself.

### What if a broker has a CAPTCHA?

The app pauses and shows you a prompt. You solve the CAPTCHA in the browser window, click "Done" in the app, and it continues. This is intentional — we don't bypass CAPTCHAs.

### What if a broker requires email verification?

Same pattern. The app tells you to check your email and click the confirmation link. Once you've done that, click "Done" and the app logs it as confirmed.

### What if a broker changes their opt-out form?

The community submits updated playbooks. Once reviewed and signed by the Opt-Outta team, the new playbook is available to all users without an app update.

### Does this work for GDPR requests?

The app is currently focused on US data brokers, but the same architecture works for GDPR Article 17 (right to erasure) requests. Broker contributions for EU data processors are welcome.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for details on:

- Adding new broker definitions to `registry/brokers.json`
- Creating and submitting playbooks
- Reporting broken broker flows
- Code contributions
- Development setup

---

## Support the Project

Opt-Outta is free and open source. If it's saved you time or you believe in the mission, consider supporting development:

- [GitHub Sponsors](https://github.com/sponsors/daringdesigns)
- [Buy Me a Coffee](https://buymeacoffee.com/daringdesigns)

---

## License

MIT — do whatever you want with it.
