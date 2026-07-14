# Microsoft Store (MSIX) packaging

This folder packages RhyseGaming Server Manager as an **MSIX** for the Microsoft Store.
The Store signs the package for you, so there's **no code-signing cert to buy** and
**no SmartScreen warning** for Store installs.

## Files

- `AppxManifest.xml` — the package manifest (with `__TOKENS__` for Partner Center values).
- `assets/` — Store logos/tiles (generated from the app icon).
- `build-msix.ps1` — stages files, fills the manifest, runs `makeappx`, and (in test mode) signs.
- `staging/`, `out/` — build output (git-ignored).

## Prerequisites

1. A release build exists: run `npm run tauri build` in the project root first
   (produces `src-tauri/target/release/RhyseGamingServerManager.exe`).
2. Windows SDK (`makeappx.exe` / `signtool.exe`) — already installed here.

## 1. Test it locally first (recommended)

Confirm the app works *inside* the MSIX container before submitting — this app launches
external programs (SteamCMD, the dedicated server executable) and manages files, which is
why the manifest declares `runFullTrust`.

```powershell
powershell -ExecutionPolicy Bypass -File build-msix.ps1 -Test
```

Then (once, as Admin — a fresh test build generates a new cert each time, so redo this if
you rebuild) trust the generated test cert and install:

```powershell
$pw = ConvertTo-SecureString -String 'test' -Force -AsPlainText
Import-PfxCertificate -FilePath out\test-cert.pfx -CertStoreLocation Cert:\LocalMachine\TrustedPeople -Password $pw
Import-PfxCertificate -FilePath out\test-cert.pfx -CertStoreLocation Cert:\LocalMachine\Root -Password $pw
Add-AppxPackage out\RhyseGamingServerManager.msix
```

(`Import-Certificate` doesn't work here — it can't handle a PFX's private key. If a previous
version is already installed with a different signature, `Add-AppxPackage` will refuse with
`0x80073CFB`; run `Get-AppxPackage *RhyseGamingServerManager* | Remove-AppxPackage` first.)

Launch from the Start menu and verify: **install server, start/stop, config, backups** all
work. (Verified 2026-07: the app's data dir resolves to the normal, non-virtualized
`AppData\Roaming\<identifier>\` path under MSIX package identity — it is *not* redirected to
per-package virtualized storage the way some MSIX docs suggest it might be.)

## 2. Build for the Store

1. Create a **Microsoft Partner Center** account (company account, verified via your
   rhysegaming.com domain/email — ~$99 one-time).
2. **Reserve the app name**: "RhyseGaming Server Manager". Being game-neutral, it sidesteps
   the trademark-wording problem a Palworld-specific name would've had — but the Store
   listing description still needs the "not affiliated with/endorsed by" disclaimer for
   Palworld, ARK: Survival Ascended, and Enshrouded (see `SettingsPage.tsx`'s About card for
   the wording already used in-app).
3. In Partner Center → **Product identity**, copy these three values:
   - Package/Identity/**Name**
   - Package/Identity/**Publisher** (`CN=...`)
   - Package/Properties/**PublisherDisplayName**
4. Build the package with those values (do **not** sign — the Store signs it):

   ```powershell
   powershell -ExecutionPolicy Bypass -File build-msix.ps1 `
     -PackageName "<Identity Name>" `
     -PublisherId "<CN=...>" `
     -PublisherDisplayName "Rhyse Gaming" `
     -AppDisplayName "<your reserved name>"
   ```

5. Upload `out\RhyseGamingServerManager.msix` in your submission, fill the listing
   (description, screenshots — reuse the ones in the website `img/` folder), and submit.
   Certification takes a few days.

## Notes

- **`runFullTrust`** is a restricted capability; it's standard for repackaged desktop apps
  and normally approved, but expect the review to look a little closer.
- Local testing (step 1) matters regardless — it's the only way to confirm the packaged app
  actually behaves the same as the unpackaged one on a given machine.
- You can keep offering the **GitHub/website `.exe` download** alongside the Store version.
- Bump `Version` (e.g. `-Version "0.1.1.0"`) for each Store update; the last digit stays `0`.
