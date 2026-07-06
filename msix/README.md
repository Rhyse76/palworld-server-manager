# Microsoft Store (MSIX) packaging

This folder packages Palworld Server Manager as an **MSIX** for the Microsoft Store.
The Store signs the package for you, so there's **no code-signing cert to buy** and
**no SmartScreen warning** for Store installs.

## Files

- `AppxManifest.xml` — the package manifest (with `__TOKENS__` for Partner Center values).
- `assets/` — Store logos/tiles (generated from the app icon).
- `build-msix.ps1` — stages files, fills the manifest, runs `makeappx`, and (in test mode) signs.
- `staging/`, `out/` — build output (git-ignored).

## Prerequisites

1. A release build exists: run `npm run tauri build` in the project root first
   (produces `src-tauri/target/release/PalworldServerManager.exe`).
2. Windows SDK (`makeappx.exe` / `signtool.exe`) — already installed here.

## 1. Test it locally first (recommended)

Confirm the app works *inside* the MSIX container before submitting — this app launches
external programs (SteamCMD, PalServer.exe) and manages files, which is why the manifest
declares `runFullTrust`.

```powershell
powershell -ExecutionPolicy Bypass -File build-msix.ps1 -Test
```

Then (once, as Admin) trust the generated test cert and install:

```powershell
Import-Certificate -FilePath out\test-cert.pfx -CertStoreLocation Cert:\LocalMachine\TrustedPeople
```

Double-click `out\PalworldServerManager.msix` to install, launch from the Start menu, and
verify: **install server, start/stop, config, backups** all work.

## 2. Build for the Store

1. Create a **Microsoft Partner Center** account (company account, verified via your
   rhysegaming.com domain/email — ~$99 one-time).
2. **Reserve the app name.** The Store is strict about trademarks, so a name like
   *"Server Manager for Palworld (Unofficial)"* is safer than "Palworld Server Manager".
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

5. Upload `out\PalworldServerManager.msix` in your submission, fill the listing
   (description, screenshots — reuse the ones in the website `img/` folder), and submit.
   Certification takes a few days.

## Notes

- **`runFullTrust`** is a restricted capability; it's standard for repackaged desktop apps
  and normally approved, but expect the review to look a little closer.
- Under MSIX, the app's data dir is redirected to the package's virtualized storage. That's
  fine, but it's why local testing (step 1) matters.
- You can keep offering the **GitHub/website `.exe` download** alongside the Store version.
- Bump `Version` (e.g. `-Version "0.1.1.0"`) for each Store update; the last digit stays `0`.
