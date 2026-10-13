# Dimo Benutzerhandbuch

Dimo versieht jedes zu prüfende Merkmal einer technischen Zeichnung mit einem nummerierten
Ballon, führt die Merkmalsliste, nimmt die Werte auf, die du an deinen Teilen gemessen hast, und
entscheidet über i.O. oder n.i.O. Es exportiert ein Prüfblatt, einen Prüfbericht, ein PDF mit
Ballons und eine Merkmalsliste. Alles bleibt auf deinem Computer.

Diese Version ist für das **unterstützte Ballonieren**: Du ziehst einen Rahmen um ein Maß und Dimo
liest seinen PDF-Text, oder du klickst ein Merkmal an und tippst den Wert so, wie er aufgedruckt
ist. Dimo liest Nennmaß, Passung und aufgedruckte Toleranzen aus dem Text, bestimmt die
Grenzwerte (zuerst aufgedruckte Toleranzen, dann die Passungstabelle nach ISO 286, dann
Allgemeintoleranz und Regeln des Projekts), erklärt, woher sie stammen, und übernimmt
Nummerierung, Liste, Speichern und Exporte. Dimo durchsucht die Zeichnung noch nicht
selbstständig. Ohne Allgemeintoleranz in den Projekteinstellungen wird ein Maß ohne Toleranz als
"Keine Toleranz festgelegt" markiert; wähle eine in den **Einstellungen** oder trage seine
Grenzwerte in der Merkmalstabelle ein.

## Seiten

| Seite | Inhalt |
|---|---|
| [Eine Zeichnung ansehen](drawing-view.md) | Zoomen, Verschieben, Blätter, Seitenleiste |
| [Eine Zeichnung ballonieren](ballooning.md) | Ballons setzen, auswählen, verschieben, gestalten und löschen |
| [Merkmalstabelle](characteristics.md) | Werte, Auswahl, Bearbeiten, Reihenfolge ändern |
| [Blätter: Drehung, Einheit und Maßstab](sheets.md) | Eigenschaften jedes Blatts |
| [Nummerierungsstrategien, Zonen und Ansichten](numbering.md) | Nach Zone, Ansicht oder Art nummerieren, Vorschau, Unternummern, gesperrte Nummerierung |
| [Projekt- und Berichtsdaten](project-details.md) | Teil, Kunde, Änderungsstand, Zeichnungs-Hash, Berichtskopf und Freigabefelder |
| [Ergebnisse eingeben](measuring.md) | Teile, Messwerttabelle, Eingabe mit der Tastatur, Einfügen, Markierungen für i.O. und n.i.O., Ergebnisarten |
| [Projekte](projects.md) | Neu, öffnen, speichern, automatische Sicherung, Wiederherstellung |
| [Exporte](exports.md) | Prüfblatt und Prüfbericht, PDF mit Ballons, CSV- und Excel-Liste, ausgegebene Zeichnungen und Berichte |
| [Toleranzen](tolerances.md) | Regeln, Allgemeintoleranz, eigene Tabellen, Erklärungen, Entwurfstabellen, Verlauf |
| [Einstellungen](settings.md) | Farbschema, Sprache, Benutzername, Firmenname, Ballonstil, Toleranzen |
| [Tastenkürzel](shortcuts.md) | Alle Tasten für macOS sowie Windows und Linux |

Drücke in der App jederzeit **?**, um die Tastenkürzel zu sehen.

## Erste Schritte: deine erste Zeichnung ballonieren

Diese Anleitung führt eine PDF-Zeichnung vom ersten Klick bis zu den exportierten Dateien. Sie
verwendet jede Funktion dieser Version einmal, in der Reihenfolge, in der du sie normalerweise
brauchst. Die Tasten sind für macOS geschrieben; unter Windows und Linux nimmst du Ctrl statt ⌘
und Alt statt ⌥.

### 1. Ein Projekt aus einer Zeichnung anlegen

Starte Dimo und wähle **Neues Projekt** (⌘N). Wähle das PDF der Zeichnung. Dimo kopiert die
Zeichnung in das Projekt, das Projekt hängt danach also nicht mehr von der Originaldatei ab, und
zeigt das erste Blatt eingepasst ins Fenster. Hat die Zeichnung mehrere Blätter (Seiten), erscheint
in der Werkzeugleiste eine Blattauswahl.

Das Projekt hat noch keine Datei. Die Werkzeugleiste zeigt "Unbenannt"; du speicherst es in
Schritt 9, aber Dimo hält schon jetzt eine wiederherstellbare Kopie von allem, was du tust
([Projekte](projects.md)).

Liegt das Blatt quer, stelle jetzt Drehung, Einheit und Maßstab unter **Blatteigenschaften**
rechts ein, wie in Schritt 8. Wer Einheit und Maßstab früh setzt, vergisst sie nicht.

### 2. Sich umsehen

Zoome mit dem Mausrad oder durch Spreizen und Zusammenziehen auf dem Trackpad (der Punkt unter dem
Zeiger bleibt, wo er ist), oder mit **+** und **-**. **0** passt das ganze Blatt wieder ins
Fenster ein. Verschiebe die Zeichnung mit den Pfeiltasten, mit der mittleren Maustaste oder mit
gedrückter Leertaste und Ziehen. Bild auf und Bild ab wechseln die Blätter. Einzelheiten:
[Eine Zeichnung ansehen](drawing-view.md).

### 3. Die Allgemeintoleranz des Projekts wählen

Die meisten Zeichnungen geben im Schriftfeld eine Allgemeintoleranz an, zum Beispiel
ISO 2768-m. Dimo liest das Schriftfeld noch nicht, du sagst es ihm also einmal: Öffne die
**Einstellungen** (⌘,), gehe zu **Toleranzen des Projekts**, wähle **ISO 2768-1** als
**Allgemeintoleranz** und die Klasse aus der Zeichnung und kehre mit ⌘1 zur Zeichnung zurück.
Ein Maß ohne Toleranz erhält seine Grenzwerte danach aus dieser Tabelle. Ohne Allgemeintoleranz
wird so ein Maß als **Keine Toleranz festgelegt** markiert und hat keine Grenzwerte. Die mit Dimo
gelieferten Tabellen tragen die Markierung **Entwurf**, bis ihre Werte geprüft sind.
Einzelheiten: [Toleranzen](tolerances.md).

### 4. Ballons setzen und Werte erfassen

1. Drücke **B** (das Werkzeug **Ballon setzen**).
2. Ziehe einen Rahmen um den Maßtext. Eine Karte zeigt, was Dimo gelesen hat: Art, Nennmaß,
   Grenzwerte, die Regel, aus der die Grenzwerte stammen, und eine Erklärung. **Enter**
   übernimmt sie. Oder klicke das Maß oder den Hinweis an, tippe den Wert so, wie er aufgedruckt
   ist, zum Beispiel `Ø8 f7`, und drücke **Enter**. Dimo liest getippten Text wie einen Rahmen.
3. **Escape** verwirft eine Karte, die du nicht willst.
4. Ziehe oder klicke das nächste Merkmal an. Du kannst gleich nach dem Klick oder dem Rahmen
   tippen.

Die Ballons werden in der Reihenfolge des Setzens mit 1, 2, 3 nummeriert. Hast du dich vertan,
drücke ⌘Z. Mit **V** wechselst du zurück zum Werkzeug **Auswählen**. Einzelheiten:
[Eine Zeichnung ballonieren](ballooning.md). Klicke einen Ballon oder eine Zeile, um Regel,
Erklärung und Verlauf eines Merkmals in der Seitenleiste zu sehen ([Toleranzen](tolerances.md)).

### 5. Die Werte in der Merkmalstabelle vervollständigen

Die Tabelle unter der Zeichnung hat eine Zeile pro Ballon. Der gelesene oder getippte Text steht in
der Spalte **Anforderung**, Art, Nennmaß, Passung und aufgedruckte Toleranzen sind schon
ausgefüllt. Ergänze, was die Prüfung braucht: **Art**, **Nennmaß**, **Oberes Abmaß** und
**Unteres Abmaß** (Dimo berechnet **Obere Grenze** und **Untere Grenze**), **Einheit**,
**Passung**, **Anzahl**, **Klasse** sowie die Spalten für Prüfmethode, Prüfmittel, Stichprobe und
Häufigkeit. Doppelklicke eine Zelle oder beginne zu tippen, bestätige mit Enter, gehe mit Tab
weiter. Nimm bei Hilfsmaßen und theoretisch genauen Maßen, die nicht gemessen werden, das Häkchen
bei **Prüfen** weg. Zahlen tippst du mit Punkt, und die Ziffern, die du tippst, werden genau so
gespeichert und exportiert (`90.0` bleibt `90.0`). Einzelheiten:
[Merkmalstabelle](characteristics.md).

Klicke eine Zeile, um ihren Ballon in der Zeichnung zu sehen, und klicke einen Ballon, um seine
Zeile zu sehen.

### 6. Die Reihenfolge und die Nummerierung ändern

Ziehe den Griff ⠿ einer Zeile, oder wähle Zeilen aus und drücke ⌥↑ oder ⌥↓. Alle Merkmale werden
neu nummeriert, und die Ballons auf der Zeichnung folgen. Ein ⌘Z stellt alles wieder her.

Um nach Zone, Ansicht oder Art zu nummerieren, nimm den Bereich **Nummerierung** in der
Seitenleiste: Wähle eine Strategie, setze den Haken bei **Neue Nummern als Vorschau zeigen**, um das
Ergebnis auf der Zeichnung und in der Tabelle zu sehen, und wähle **Nummerierung anwenden**.
Zonenraster und Ansichten zeichnest du von Hand unter **Blatteigenschaften**. Einzelheiten:
[Nummerierungsstrategien, Zonen und Ansichten](numbering.md).

### 7. Die Ballons gestalten

Wähle Ballons aus und drücke **S**: Form (Kreis, Fahne, Rechteck), Bezugslinie, Größe und Randfarbe.
Der Stil des ganzen Projekts steht unter **Einstellungen**. Der Status wird neben der Farbe auch
über die Form gezeigt. Einzelheiten: [Eine Zeichnung ballonieren](ballooning.md),
[Einstellungen](settings.md).

### 8. Drehung, Einheit und Maßstab des Blatts einstellen

Unter **Blatteigenschaften** rechts: Drehe das Blatt in 90-Grad-Schritten (**R** und
**Shift+R**), wähle Millimeter oder Zoll und den Maßstab aus dem Schriftfeld. Jedes Blatt hat
eigene Werte. Einzelheiten: [Blätter](sheets.md).

### 9. Speichern

Wähle **Speichern** (⌘S). Da das Projekt noch keine Datei hat, fragt Dimo nach Name und Ort und
schreibt eine `.dimo`-Datei, die die Zeichnung, die Merkmale, die Ballons und das Protokoll aller
Änderungen enthält. Danach speichert ⌘S in diese Datei. **Speichern unter** (⇧⌘S) speichert unter
einem anderen Namen.

Zwischen dem Speichern schreibt Dimo jede Änderung in ein Sicherungsjournal. Nach einem Absturz oder
Stromausfall stellt das erneute Öffnen des Projekts (oder der Start von Dimo, bei einem nie
gespeicherten Projekt) die Arbeit wieder her, mit einem Hinweis über der Zeichnung. Beim Schließen
des Fensters oder beim Anlegen oder Öffnen eines anderen Projekts mit ungespeicherten Änderungen
fragt Dimo zuerst nach. Einzelheiten: [Projekte](projects.md).

### 10. Die Ergebnisse eingeben

Öffne die Ansicht **Messen** (⌘3). Gib unter **Anzahl** ein, wie viele Teile du misst, und wähle
**Teile hinzufügen**. Klicke auf die erste Zelle eines Teils, tippe den Wert, drücke **Enter** und
mach in der Spalte nach unten weiter: Dimo speichert den Wert, geht zum nächsten Merkmal und
markiert das Ergebnis mit ✓ (i.O.), ✗ (n.i.O.) oder ○ (Wert ohne Grenzmaße). Bei Gewinden und
Hinweisen drückst du **O** für i.O. oder **N** für n.i.O. Ein aus einer Tabellenkalkulation
kopierter Bereich wird mit ⌘V eingefügt. Die Grenzmaße gehören dazu, und Dimo entscheidet für dich
über i.O. oder n.i.O. Einzelheiten: [Ergebnisse eingeben](measuring.md).

### 11. Exportieren

Trage zuerst Teil, Änderungsstand der Zeichnung und die Berichtsfelder unter **Projekt und Bericht**
ein ([Projekt- und Berichtsdaten](project-details.md)). Öffne dann die Ansicht **Export** (⌘E).

- **Prüfblatt** (PDF oder Excel): die Merkmale mit leeren Ergebnisspalten, zum Ausdrucken vor dem
  Messen.
- **Prüfbericht** (PDF oder Excel): Übersicht, Teile, Ergebnisse mit Bewertungen und
  Unterschriftenblock.
- **PDF mit Ballons**: die Zeichnung mit eingezeichneten Ballons. Die Originaldatei bleibt
  unverändert.
- **Merkmalsliste (CSV)** und **(Excel)**: eine Zeile pro Merkmal, in Ballonreihenfolge, mit der
  Regel, die jedes Grenzmaß ergeben hat.

Wähle die Sprache der Berichte und Spaltenüberschriften und wie das PDF die Ballons enthält. Jeder
Export fragt, wo die Datei gespeichert wird, und läuft im Hintergrund; **Export abbrechen** stoppt
einen Berichtsexport. Setze für eine Zeichnung oder einen Bericht, der an einen Kunden geht,
vorher das Häkchen bei **Als ausgegeben exportieren**: Es sperrt die Nummerierung, die Nummern
ändern sich danach nie mehr, auch wenn du Merkmale hinzufügst oder löschst, und ein Bericht wird im
Projekt vermerkt. Einzelheiten: [Exporte](exports.md).

### 12. Einstellungen

Unter **Einstellungen** (⌘,) wählst du das Farbschema (hell, dunkel oder wie das System), die
Sprache der App, den Benutzernamen, der in das Änderungsprotokoll jedes Projekts geschrieben wird,
und den Firmennamen für Berichte. Die Einstellungen bleiben für den nächsten Start erhalten. Die Toleranzregeln des Projekts
(Schritt 3) stehen in derselben Ansicht. Einzelheiten: [Einstellungen](settings.md).

### Wie es weitergeht

- Öffne das Projekt später mit **Projekt öffnen** (⌘O) und mache weiter. Jede Änderung, die du
  gemacht hast, steht im Änderungsprotokoll im Projekt.
- Gib die gemessenen Werte deiner Teile in der Ansicht **Messen** (⌘3) ein, siehe
  [Ergebnisse eingeben](measuring.md).
- Ändern sich Grenzmaße, nachdem du Ergebnisse eingegeben hast, bleiben die Ergebnisse, und die
  Bewertungen folgen den neuen Grenzmaßen
  ([Ergebnisse eingeben](measuring.md#wenn-sich-grenzmaße-ändern)).
- Noch nicht in dieser Version: automatisches Durchsuchen der Zeichnung. Die Ansicht **Prüfen**
  zeigt einen Hinweis, dass sie in einer späteren Version folgt.
