# Datei-Seite, Optionen und Über Dimo

Das Register **Datei** öffnet eine ganze Seite. Links stehen die Befehle für das Projekt und die
Bereiche der Seite, rechts der Bereich, den du gewählt hast. **Zurück** (oder ein anderes
Register) bringt dich dahin, wo du warst.

| Eintrag | Was er tut |
|---|---|
| **Info** | Die Daten, die mit dem Projekt gespeichert werden, siehe unten |
| **Neues Projekt** | Startet ein Projekt aus einer PDF-Zeichnung (⌘N) |
| **Projekt öffnen** | Zeigt die zuletzt geöffneten Projekte und **Projekt öffnen** für jede andere Datei (⌘O) |
| **Speichern**, **Speichern unter…** | Speichern das Projekt ([Projekte](projects.md)) |
| **Exportieren** | Öffnet das Register **Bericht** ([Exporte](exports.md)) |
| **Drucken** | Nicht in dieser Version: Speichere im Register Bericht eine Kopie und drucke sie aus deinem PDF-Programm |
| **Projekt schließen** | Schließt das Projekt, Dimo bleibt offen. Bei ungespeicherten Änderungen fragt Dimo zuerst |
| **Über Dimo** | Version und Lizenzen, siehe unten |
| **Optionen** | Deine persönlichen Einstellungen (⌘,), siehe unten |

Jede Einstellung hat einen Ort. Einstellungen eines Projekts werden in der Projektdatei
gespeichert und reisen mit ihr: die Allgemeintoleranz (Register Start und Zeichnung), die übrigen
Toleranzregeln und der Ballonstil (Register Zeichnung) sowie die Teil-, Zeichnungs- und
Berichtsangaben (Datei > Info und die Seitenleiste). Deine persönlichen Einstellungen stehen unter
**Optionen** und werden nie im Projekt gespeichert.

## Info

**Projektinfo** zeigt, was mit dem offenen Projekt gespeichert wird. Jede Änderung ist ein
Rückgängig-Schritt.

- **Teil**: Teilenummer, Teilebezeichnung, Kunde und Auftragsnummer.
- **Toleranzen**: die Allgemeintoleranz, die Regeln nach Dezimalstellen und die Firmentabellen
  des Projekts. **Im Register Zeichnung ändern** bringt dich dorthin, wo du sie änderst.
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
Projekt; bei ungespeicherten Änderungen fragt Dimo zuerst. Ein Projekt, dessen Datei verschoben,
umbenannt oder auf einem nicht verbundenen Laufwerk ist, erscheint grau und durchgestrichen mit
„Datei nicht gefunden“. Mit ✕ entfernst du einen Eintrag (die Datei selbst bleibt unberührt).

Dimo merkt sich nur die Pfade der Dateien, in einer kleinen Datei neben den Einstellungen im
Konfigurationsordner von Dimo deines Benutzerkontos. Nichts verlässt deinen Computer.

## Optionen

Öffne **Optionen** unten auf der Datei-Seite oder mit ⌘, (Strg+, unter Windows und Linux).
Änderungen gelten sofort und bleiben für den nächsten Start erhalten. Diese Einstellungen gehören
dir, nicht einem Projekt: Sie gelten für jedes Projekt und liegen in einer kleinen
Einstellungsdatei im Konfigurationsordner von Dimo deines Benutzerkontos.

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
Anzahl leerer Ergebnisspalten, wie das PDF die Ballons trägt und ob Dimo eine Datei nach dem
Export öffnet. Dimo merkt sich, was du zuletzt im Register Bericht gewählt hast; du kannst es
hier oder dort ändern ([Exporte](exports.md)).

## Projekteinstellungen im Register Zeichnung

- **Allgemeintoleranz**: ein Feld in den Registern Start und Zeichnung ([Toleranzen](tolerances.md)).
- **Dezimalregeln…**: ein kleines Fenster mit den Regeln nach Dezimalstellen und der Rundung nach
  Einheitenumrechnung.
- **Firmentabellen**: ein kleines Fenster mit den eigenen Tabellen des Projekts, **Tabelle
  importieren…** und der Zeichnungsregel.
- **Ballonstil des Projekts**: ein kleines Fenster mit Form, Größe, Linienbreite, Randfarbe und
  Bezugslinie aller Ballons des Projekts ohne eigenen Stil. Einzelnen Ballons gibst du einen
  eigenen Stil, indem du sie in der Zeichnung auswählst und **Stil** (S) verwendest.

Jede Änderung in diesen Fenstern gilt sofort und ist ein Rückgängig-Schritt. **Schließen**
schließt das Fenster.

## Über Dimo

**Über Dimo** zeigt die Version von Dimo, seine Lizenz (Apache License 2.0) und die Datei NOTICE.
Dazu kommt jede Schrift und jeder Symbolsatz, den Dimo mitbringt (Open Sans, Space Grotesk,
JetBrains Mono und die Lucide-Symbole), mit Copyright-Hinweis und vollständigem Lizenztext. Die
Texte sind Teil der App und erscheinen daher auch ohne Internetverbindung. Lizenztexte stehen im
englischen Original. Die Lizenzen der übrigen Open-Source-Bibliotheken, mit denen Dimo gebaut ist,
stehen in der Datei `THIRD_PARTY_LICENSES`, die jeder Version beiliegt.
