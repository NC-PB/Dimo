# Datei-Seite, Optionen und Über DimoMark

Der Reiter **Datei** öffnet eine ganze Seite. Links stehen die Befehle für das Projekt und die
Bereiche der Seite, rechts der Bereich, den du gewählt hast. **Zurück** (oder ein anderer
Reiter) bringt dich dahin, wo du warst.

| Eintrag | Was er tut |
|---|---|
| **Info** | Die Daten, die mit dem Projekt gespeichert werden, siehe unten |
| **Neues Projekt** | Startet ein Projekt aus einer PDF-Zeichnung (⌘N) |
| **Projekt öffnen** | Zeigt die zuletzt geöffneten Projekte und **Projekt öffnen** für jede andere Datei (⌘O) |
| **Speichern**, **Speichern unter…** | Speichern das Projekt ([Projekte](projects.md)) |
| **Exportieren** | Öffnet den Reiter **Bericht** ([Exporte](exports.md)) |
| **Drucken** | Öffnet den im Reiter Bericht gewählten Bericht in deinem PDF-Programm; dort druckst du ihn ([Exporte](exports.md)) |
| **Projekt schließen** | Schließt das Projekt, DimoMark bleibt offen. Bei ungespeicherten Änderungen fragt DimoMark zuerst |
| **Über DimoMark** | Version und Lizenzen, siehe unten |
| **Optionen** | Deine persönlichen Einstellungen (⌘,), siehe unten |

Die Version von DimoMark steht unten in der Liste links.

Jede Einstellung hat einen Ort. Einstellungen eines Projekts werden in der Projektdatei
gespeichert und reisen mit ihr: die Allgemeintoleranz (Reiter Start und Zeichnung), die übrigen
Toleranzregeln und der Ballonstil (Reiter Zeichnung) sowie die Teil-, Zeichnungs- und
Berichtsangaben (Datei > Info). Deine persönlichen Einstellungen stehen unter
**Optionen** und werden nie im Projekt gespeichert.

## Info

**Projektinfo** zeigt, was mit dem offenen Projekt gespeichert wird. Jede Änderung ist ein
Rückgängig-Schritt.

- **Teil**: Teilenummer, Teilebezeichnung, Kunde und Auftragsnummer.
- **Toleranzen**: das Feld **Allgemeintoleranz** (dasselbe wie in den Reitern Start und
  Zeichnung) sowie die Dezimalregeln und die Firmentabellen des Projekts. **Im
  Reiter Zeichnung ändern** bringt dich dorthin, wo du die Regeln änderst.
- **Zeichnung**: die Zeichnungsdatei mit der Anzahl Blätter, der Änderungsstand und der
  SHA-256-Fingerabdruck der Datei mit **Kopieren**.
- **Fortschritt**: die Anzahl Merkmale und zu prüfender Merkmale, wie viele Teile für jedes
  Merkmal ein Ergebnis haben, das Gesamtergebnis und der letzte als ausgegeben exportierte Bericht.
- **Bericht**: Berichtsnummer, Firma, geprüft von, freigegeben von und das Berichtsdatum
  (**Heute** setzt das Datum). Einzelheiten: [Projekt- und Berichtsdaten](project-details.md).

Ohne offenes Projekt sagt Info das und bietet **Neues Projekt** und **Projekt öffnen** an.

## Zuletzt geöffnete Projekte

**Projekt öffnen** auf der Datei-Seite listet die Projekte, die du zuletzt geöffnet oder unter
einem neuen Namen gespeichert hast, das neueste zuerst, höchstens zehn. Ein Klick öffnet ein
Projekt; bei ungespeicherten Änderungen fragt DimoMark zuerst. Ein Projekt, dessen Datei verschoben,
umbenannt oder auf einem nicht verbundenen Laufwerk ist, erscheint grau und durchgestrichen mit
„Datei nicht gefunden“. Mit ✕ entfernst du einen Eintrag (die Datei selbst bleibt unberührt).

DimoMark merkt sich nur die Pfade der Dateien, in einer kleinen Datei neben den Einstellungen im
Konfigurationsordner von DimoMark deines Benutzerkontos. Nichts verlässt deinen Computer.

## Optionen

Öffne **Optionen** unten auf der Datei-Seite oder mit ⌘, (Strg+, unter Windows und Linux).
Änderungen gelten sofort und bleiben für den nächsten Start erhalten. Diese Einstellungen gehören
dir, nicht einem Projekt: Sie gelten für jedes Projekt und liegen in einer kleinen
Einstellungsdatei im Konfigurationsordner von DimoMark deines Benutzerkontos.

### Darstellung

- **Farbschema**: wie das System, hell oder dunkel. Der Mond- oder Sonnenknopf in der Titelleiste
  wechselt ebenfalls zwischen hell und dunkel.
- **Sprache**: wie das System, Englisch oder Deutsch. Beim Wechsel der Sprache wird das Fenster
  neu geladen; das offene Projekt bleibt offen.

### Du und deine Firma

- **Benutzername**: wird bei jeder Änderung an einem Projekt festgehalten. Leer lassen, um den
  Benutzernamen des Betriebssystems zu verwenden, den das Feld grau anzeigt.
- **Firmenname**: wird auf Berichten gedruckt. Neue Projekte beginnen damit in ihren
  Berichtsdaten. Bestehende Projekte behalten ihren Namen ([Projekt- und Berichtsdaten](project-details.md)).
  Ein als ausgegeben exportierter Bericht trägt ihn in ein Projekt ohne Firmennamen ein
  ([Exporte](exports.md)).

### Vorgaben für Exporte

Die Sprache der Berichte und Spaltenköpfe, das Dateiformat von Prüfblatt und Prüfbericht, die
Anzahl leerer Ergebnisspalten, wie das PDF die Ballons trägt und ob DimoMark eine Datei nach dem
Export öffnet. DimoMark merkt sich, was du zuletzt im Reiter Bericht gewählt hast; du kannst es
hier oder dort ändern ([Exporte](exports.md)).

## Projekteinstellungen im Reiter Zeichnung

- **Allgemeintoleranz**: ein Feld in den Reitern Start und Zeichnung und unter Datei > Info
  ([Toleranzen](tolerances.md)).
- **Dezimalregeln…**: ein kleines Fenster mit den Dezimalregeln und der Rundung nach
  Einheitenumrechnung.
- **Firmentabellen**: ein kleines Fenster mit den Firmentabellen des Projekts, **Tabelle
  importieren…** und der Zeichnungsregel.
- **Ballonstil des Projekts**: ein kleines Fenster mit Form, Größe, Linienbreite, Randfarbe und
  Bezugslinie aller Ballons des Projekts ohne eigenen Stil. Einzelnen Ballons gibst du einen
  eigenen Stil, indem du sie in der Zeichnung auswählst und **Stil** (S) verwendest.

Jede Änderung in diesen Fenstern gilt sofort und ist ein Rückgängig-Schritt. **Schließen**
schließt das Fenster.

## Über DimoMark

**Über DimoMark** zeigt die Version von DimoMark, seine Lizenz (Apache License 2.0) und die Datei NOTICE.
Dazu kommt jede Schrift und jeder Symbolsatz, den DimoMark mitbringt (Open Sans, Space Grotesk,
JetBrains Mono und die Lucide-Symbole), mit Copyright-Hinweis und vollständigem Lizenztext. Die
Texte sind Teil der App und erscheinen daher auch ohne Internetverbindung. Lizenztexte stehen im
englischen Original.

Unter **Open-Source-Bibliotheken** folgt die Liste aller übrigen Bestandteile, mit denen DimoMark gebaut
ist: die Rust-Crates des Programms, die Pakete der Benutzeroberfläche und die PDF-Bibliothek PDFium.
Jeder steht mit Version und Copyright-Hinweis da, nach Lizenz gruppiert, und jeder Lizenztext
erscheint einmal. Der letzte Teil nennt die Systembibliotheken, die das Linux-AppImage mitbringt (GTK,
WebKitGTK und weitere aus Ubuntu 22.04), jede mit ihrer Debian-Copyright-Datei; die anderen Versionen
enthalten sie nicht. **Kopieren** legt die ganze Liste in die Zwischenablage. Dieselbe Liste liegt jedem
Installationsprogramm als Datei `THIRD_PARTY_LICENSES` bei, im Ordner `licenses` der installierten
App; in der portablen Version liegt sie neben `DimoMark.exe`.

## Protokolldatei

DimoMark schreibt Meldungen wie Warnungen und Fehler (zum Beispiel eine fehlgeschlagene automatische
Sicherung, einen fehlgeschlagenen Export oder eine unlesbare Einstellungsdatei) und die gestartete
Version in eine Protokolldatei auf Ihrem Computer. Sie enthält Meldungen und Dateipfade, nicht den
Inhalt von Zeichnungen oder Projekten, und nichts wird verschickt. Erreicht sie 1 MB, wird sie in
`dimo.log.1` umbenannt und eine neue Datei beginnt. Wenn Sie ein Problem melden, hilft das Protokoll, die Ursache zu finden:

- macOS: `~/Library/Logs/io.github.nc-pb.dimo/dimo.log`
- Windows: `%LOCALAPPDATA%\io.github.nc-pb.dimo\logs\dimo.log`
- Linux: `~/.local/share/io.github.nc-pb.dimo/logs/dimo.log`

**Diagnose kopieren** in **Über DimoMark** legt einen kurzen Text für eine Fehlermeldung in die
Zwischenablage: die Version von DimoMark, das Betriebssystem, deine Optionen und die letzten 200 Zeilen
der Protokolldatei. Dein Name, der Firmenname und dein Benutzerordner werden ersetzt (`<user>`,
`<company>`, `~`). Nichts wird gesendet; füge den Text dort ein, wo du den Fehler meldest.

## Nach Update suchen

DimoMark sucht nie von sich aus nach Updates. **Nach Update suchen** in **Über DimoMark** fragt
GitHub einmal nach der neuesten veröffentlichten Version, nur wenn du darauf klickst. Die Anfrage
geht an `api.github.com` und enthält nur, was die Anfrage selbst braucht: die Version von DimoMark
als Programmname (`DimoMark/0.1.0`). Nichts wird heruntergeladen oder installiert.

- Eine neuere Version: DimoMark zeigt ihre Nummer, und **Release-Seite öffnen** öffnet die Seite in
  deinem Browser. Lade sie herunter und installiere sie selbst, wie beim ersten Mal.
- Dieselbe Version: „Du hast die neueste Version“.
- Keine Internetverbindung, oder GitHub ist nicht erreichbar: eine Meldung, sonst passiert nichts.

DimoMark verwendet einen Proxyserver, wenn eine der üblichen Umgebungsvariablen gesetzt ist
(`ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY`, auch kleingeschrieben; `NO_PROXY` nennt Hosts ohne
Proxy). In einem Netzwerk, das HTTPS mit einem eigenen Zertifikat prüft, schlägt die Suche fehl:
DimoMark vertraut nur den eingebauten Stammzertifikaten.

### Die Suche für eine Organisation abschalten

Ein Administrator kann die Schaltfläche für alle Benutzer eines Computers mit einer Richtliniendatei
ausblenden:

- macOS: `/Library/Application Support/DimoMark/policy.json`
- Windows: `%ProgramData%\DimoMark\policy.json` (meist `C:\ProgramData\DimoMark\policy.json`)
- Linux: `/etc/dimomark/policy.json`

Die Datei enthält:

```json
{ "update_check": false }
```

`true` oder eine Datei ohne `update_check` zeigt die Schaltfläche. Eine Datei, die kein gültiges
JSON ist, blendet die Schaltfläche ebenfalls aus, und der Grund steht in der Protokolldatei.
DimoMark liest die Datei, wenn **Über DimoMark** geöffnet wird, und noch einmal vor der Anfrage.
