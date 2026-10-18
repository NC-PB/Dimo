# Projekte

Ein Projekt enthält eine Zeichnung, ihre Merkmale und Ballons sowie das Protokoll aller
Änderungen. Es wird als eine `.dimo`-Datei gespeichert. Die Zeichnung liegt unverändert in der
Datei, darum öffnet sich das Projekt auch dann, wenn die Originaldatei verschoben oder gelöscht
wurde.

## Anlegen, öffnen, speichern

| Aktion | Wo | macOS | Windows und Linux |
|---|---|---|---|
| Neues Projekt aus einer PDF-Zeichnung, einem Foto oder einem Scan | Datei > Neues Projekt | ⌘N | Ctrl+N |
| Projekt öffnen | Datei > Projekt öffnen | ⌘O | Ctrl+O |
| Speichern | Speichern in der Titelleiste, oder Datei | ⌘S | Ctrl+S |
| Unter neuem Namen speichern | Datei > Speichern unter… | ⇧⌘S | Ctrl+Shift+S |
| Rückgängig | Rückgängig in der Titelleiste | ⌘Z | Ctrl+Z |
| Wiederholen | Wiederholen in der Titelleiste | ⇧⌘Z | Ctrl+Shift+Z oder Ctrl+Y |

**Neues Projekt** und **Projekt öffnen** fragen mit dem Dateidialog des Betriebssystems nach einer
Datei. Ein Projekt entsteht aus einer PDF-Zeichnung oder aus einem Foto oder Scan als JPEG, PNG
oder TIFF (siehe unten); andere Dateitypen werden nicht angeboten. Ein
neues Projekt hat bis zum ersten Speichern keine Datei; die Titelleiste zeigt "Unbenannt".
Beim ersten Speichern fragt DimoMark nach einem Dateinamen und hängt `.dimo` an, wenn du es weglässt.
Rückgängig ist unbegrenzt, solange das Projekt offen ist; nach erneutem Öffnen beginnt der Verlauf
neu.

Ohne offenes Projekt zeigt DimoMark den Startbildschirm mit **Neues Projekt aus Zeichnung**,
**Projekt öffnen** und den fünf zuletzt geöffneten Projekten; ein Klick auf eines öffnet es. Gleich nach dem Anlegen eines neuen Projekts fragt der Bereich **Zeichnung
einrichten** nach Allgemeintoleranz, Einheit und Maßstab, Teilenummer und Kunde; **Überspringen**
schließt ihn, und **Zeichnung einrichten** im Reiter **Zeichnung** öffnet ihn wieder. Geöffnete
Projekte zeigen ihn nicht. Ansichten und Bereiche ohne Inhalt (keine Merkmale, keine Teile, keine
Messwerte) sagen, was fehlt, und bieten eine Schaltfläche für den nächsten Schritt.

Hat ein Projekt ungespeicherte Änderungen, fragt DimoMark vor dem Anlegen oder Öffnen eines anderen
Projekts und vor dem Schließen des Fensters oder dem Beenden von DimoMark: **Speichern**,
**Nicht speichern** oder **Abbrechen**.

## Zeichnungen aus Fotos und Scans

Ein Foto oder Scan einer Papierzeichnung öffnet sich wie ein PDF: wähle die JPEG-, PNG- oder
TIFF-Datei unter **Neues Projekt**. Ein TIFF mit mehreren Seiten ergibt ein Blatt pro Seite.
DimoMark dreht ein Foto so, wie das Telefon es aufgenommen hat, und legt das Bild unverändert im
Projekt ab.

- **Blattgröße:** Ein Scan, der seine Auflösung angibt (100 bis 1200 dpi), bekommt seine echte
  Größe. Ein Foto kennt seine Größe nicht, darum wird seine lange Seite auf 297 mm gesetzt, die
  Länge eines A4-Blatts. Große Bilder werden bei dieser Größe mit höchstens 300 dpi gehalten.
- **Kein Text im Bild:** Ein Rahmen um ein Maß findet keinen Text, also wird der Ballon gesetzt und
  du tippst den Wert ein, wie bei einem PDF ohne Text.
- **HEIC-Fotos** (Standard auf dem iPhone) können nicht geöffnet werden. Wähle auf dem iPhone
  Einstellungen > Kamera > Formate > Maximale Kompatibilität, oder sende dir das Foto per Mail
  oder Messenger, das macht meist ein JPEG daraus.
- Merkmalsliste und Berichte funktionieren wie bei einem PDF. Das PDF mit Ballons gibt es für
  Fotos und Scans noch nicht.

## Was in einer Projektdatei steht

Eine `.dimo`-Datei enthält die Zeichnung, wie du sie importiert hast, die Merkmale und Ballons,
die Einstellungen des Projekts (Ballonstil, Toleranzregeln mit den importierten eigenen
Tabellen, Nummerierungsstrategie), Drehung, Einheit, Maßstab, Zonenraster und Ansichten jedes
Blatts, die Nummerierungssperre und das Änderungsprotokoll: jede Änderung mit Zeit und dem
Benutzernamen aus den **Optionen** ([Datei-Seite, Optionen und Über DimoMark](settings.md)). Die Datei ist ein ZIP-Archiv, du kannst sie also
mit jedem ZIP-Programm ansehen, aber nur DimoMark sollte sie schreiben.

Ein Projekt, das eine ältere DimoMark-Version gespeichert hat, lässt sich öffnen, und ein Hinweis sagt,
dass es umgewandelt wurde; beim Speichern wird das aktuelle Format geschrieben. Ein Projekt von
einer neueren DimoMark-Version wird mit einer Meldung abgelehnt, damit nichts verloren geht;
aktualisiere DimoMark, um es zu öffnen.

## Automatische Sicherung und Wiederherstellung

Jede Änderung wird sofort, spätestens aber alle 30 Sekunden, in ein Sicherungsjournal
geschrieben. Die Titelleiste zeigt den Zustand neben dem Projektnamen: "gespeichert", "noch nicht
gespeichert" (ein neues Projekt ohne Änderungen), "nicht gespeichert", "nicht gespeichert,
automatisch gesichert" oder "Automatische Sicherung fehlgeschlagen" mit dem Grund und einem
Warnzeichen, wenn das Journal nicht geschrieben werden konnte.

- Ein gespeichertes Projekt hat sein Journal neben der Datei: `part.dimo.journal`. Beim Speichern
  wird es in die Projektdatei übernommen und gelöscht.
- Ein noch nie gespeichertes Projekt hat sein Journal im Datenordner von DimoMark (unter macOS zum
  Beispiel `~/Library/Application Support/io.github.nc-pb.dimo/autosave`, unter Windows
  `%APPDATA%\io.github.nc-pb.dimo\autosave`).

Endet DimoMark oder der Computer unerwartet, gehen höchstens die letzten 30 Sekunden verloren:

- Beim erneuten Öffnen des gespeicherten Projekts werden die Änderungen aus dem Journal
  wiederhergestellt.
- Ein noch nie gespeichertes Projekt wird beim nächsten Start von DimoMark wiederhergestellt.

Ein Hinweis über der Zeichnung sagt, was wiederhergestellt wurde. Speichere das Projekt, um die
Änderungen zu behalten. Wählst du beim Schließen "Nicht speichern", wird das Journal gelöscht.

## Ein Fenster pro Projekt

Solange ein Projekt offen ist, legt DimoMark daneben eine Sperrdatei an (`part.dimo.lock`). Ein
zweites DimoMark-Fenster, das dasselbe Projekt öffnen will, erhält eine Meldung mit dem Benutzer, der
es geöffnet hat. Die Sperre endet, wenn DimoMark schließt, auch nach einem Absturz; eine
liegengebliebene Sperrdatei blockiert also nie. Die Sperre gilt nur zwischen DimoMark-Fenstern;
andere Programme können die Datei weiterhin ändern.

## Windows: WebView2, Pfade und Netzlaufwerke

DimoMark zeichnet sein Fenster mit der Microsoft Edge WebView2 Runtime. Windows 11 und
aktualisiertes Windows 10 haben sie. Das MSI-Installationsprogramm installiert sie, wenn sie
fehlt; die portable ZIP-Datei kann das nicht. Fehlt die Runtime, zeigt das portable DimoMark eine
Meldung mit der Downloadseite (`https://developer.microsoft.com/microsoft-edge/webview2/`) und
beendet sich. Installiere dort die Evergreen WebView2 Runtime oder verwende das MSI und starte
DimoMark erneut. Die portable Version braucht keine Administratorrechte; lass die Ordner `pdfium`
und `licenses` neben `DimoMark.exe`.

- **Namen**: Datei- und Ordnernamen dürfen Leerzeichen und Umlaute enthalten, zum Beispiel
  `Zeichnungen Müller\Prüfteil 1.dimo`.
- **Lange Pfade**: DimoMark öffnet, speichert und exportiert Projekte, deren Pfad länger als 260
  Zeichen ist. Der Dateidialog von Windows und das Programm, das eine exportierte Datei öffnet,
  lehnen solche Pfade unter Umständen ab, wenn in Windows "Lange Win32-Pfade aktivieren" nicht
  eingeschaltet ist (eine Einstellung für Administratoren). Wähle dann einen kürzeren Ordner.
- **Netzlaufwerke**: Projekte auf einem Netzlaufwerk öffnen und speichern sich wie lokale, über
  einen Laufwerksbuchstaben (`Z:\Projekte`) oder einen Netzwerkpfad (`\\server\share\Projekte`).
  Die Liste der zuletzt geöffneten Projekte zeigt den Pfad so, wie du ihn gewählt hast.
  Sperrdatei und Journal liegen neben dem Projekt, du brauchst also das Recht, in diesem Ordner
  Dateien anzulegen. Bricht die Verbindung ab, schlagen automatisches Speichern und Speichern mit
  einer Meldung fehl, und deine Arbeit bleibt im offenen Fenster: Speichere erneut, wenn das
  Laufwerk wieder da ist, oder speichere mit **Speichern unter** auf deinem eigenen Computer.
