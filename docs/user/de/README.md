# Dimo Benutzerhandbuch

Dimo versieht jedes zu prüfende Merkmal einer technischen Zeichnung mit einem nummerierten
Ballon, führt die Merkmalsliste und exportiert ein PDF mit Ballons und eine Merkmalsliste für die
Prüfung. Alles bleibt auf deinem Computer.

Diese Version ist für das **unterstützte Ballonieren**: Du ziehst einen Rahmen um ein Maß und Dimo
liest seinen PDF-Text, oder du klickst ein Merkmal an und tippst den Wert so, wie er aufgedruckt
ist. Dimo liest Nennmaß, Passung und aufgedruckte Toleranzen aus dem Text, bestimmt die
Grenzwerte (zuerst aufgedruckte Toleranzen, dann die Passungstabelle nach ISO 286), erklärt, woher
sie stammen, und übernimmt Nummerierung, Liste, Speichern und Exporte. Dimo durchsucht die
Zeichnung noch nicht selbstständig. Eine Allgemeintoleranz für das Projekt lässt sich in dieser
Version noch nicht wählen, ein Maß ohne Toleranz wird daher als "Keine Toleranz festgelegt"
markiert; seine Grenzwerte trägst du in der Merkmalstabelle ein.

## Seiten

| Seite | Inhalt |
|---|---|
| [Eine Zeichnung ansehen](drawing-view.md) | Zoomen, Verschieben, Blätter, Seitenleiste |
| [Eine Zeichnung ballonieren](ballooning.md) | Ballons setzen, auswählen, verschieben, gestalten und löschen |
| [Merkmalstabelle](characteristics.md) | Werte, Auswahl, Bearbeiten, Reihenfolge ändern |
| [Blätter: Drehung, Einheit und Maßstab](sheets.md) | Eigenschaften jedes Blatts |
| [Nummerierungsstrategien, Zonen und Ansichten](numbering.md) | Nach Zone, Ansicht oder Art nummerieren, Vorschau, Unternummern, gesperrte Nummerierung |
| [Projekte](projects.md) | Neu, öffnen, speichern, automatische Sicherung, Wiederherstellung |
| [Exporte](exports.md) | PDF mit Ballons, CSV- und Excel-Liste, ausgegebene Zeichnungen |
| [Einstellungen](settings.md) | Farbschema, Sprache, Benutzername, Ballonstil |
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
Schritt 8, aber Dimo hält schon jetzt eine wiederherstellbare Kopie von allem, was du tust
([Projekte](projects.md)).

Liegt das Blatt quer, stelle jetzt Drehung, Einheit und Maßstab unter **Blatteigenschaften**
rechts ein, wie in Schritt 7. Wer Einheit und Maßstab früh setzt, vergisst sie nicht.

### 2. Sich umsehen

Zoome mit dem Mausrad oder durch Spreizen und Zusammenziehen auf dem Trackpad (der Punkt unter dem
Zeiger bleibt, wo er ist), oder mit **+** und **-**. **0** passt das ganze Blatt wieder ins
Fenster ein. Verschiebe die Zeichnung mit den Pfeiltasten, mit der mittleren Maustaste oder mit
gedrückter Leertaste und Ziehen. Bild auf und Bild ab wechseln die Blätter. Einzelheiten:
[Eine Zeichnung ansehen](drawing-view.md).

### 3. Ballons setzen und Werte erfassen

1. Drücke **B** (das Werkzeug **Ballon setzen**).
2. Ziehe einen Rahmen um den Maßtext. Eine Karte zeigt, was Dimo gelesen hat; **Enter**
   übernimmt sie. Oder klicke das Maß oder den Hinweis an, tippe den Wert so, wie er aufgedruckt
   ist, zum Beispiel `Ø8 f7`, und drücke **Enter**.
3. **Escape** verwirft eine Karte, die du nicht willst.
4. Klicke das nächste Merkmal an.

Die Ballons werden in der Reihenfolge des Setzens mit 1, 2, 3 nummeriert. Hast du dich vertan,
drücke ⌘Z. Mit **V** wechselst du zurück zum Werkzeug **Auswählen**. Einzelheiten:
[Eine Zeichnung ballonieren](ballooning.md).

### 4. Die Werte in der Merkmalstabelle vervollständigen

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

### 5. Die Reihenfolge ändern

Ziehe den Griff ⠿ einer Zeile, oder wähle Zeilen aus und drücke ⌥↑ oder ⌥↓. Alle Merkmale werden
neu nummeriert, und die Ballons auf der Zeichnung folgen. Ein ⌘Z stellt alles wieder her.

### 6. Die Ballons gestalten

Wähle Ballons aus und drücke **S**: Form (Kreis, Fahne, Rechteck), Bezugslinie, Größe und Randfarbe.
Der Stil des ganzen Projekts steht unter **Einstellungen**. Der Status wird neben der Farbe auch
über die Form gezeigt. Einzelheiten: [Eine Zeichnung ballonieren](ballooning.md),
[Einstellungen](settings.md).

### 7. Drehung, Einheit und Maßstab des Blatts einstellen

Unter **Blatteigenschaften** rechts: Drehe das Blatt in 90-Grad-Schritten (**R** und
**Shift+R**), wähle Millimeter oder Zoll und den Maßstab aus dem Schriftfeld. Jedes Blatt hat
eigene Werte. Einzelheiten: [Blätter](sheets.md).

### 8. Speichern

Wähle **Speichern** (⌘S). Da das Projekt noch keine Datei hat, fragt Dimo nach Name und Ort und
schreibt eine `.dimo`-Datei, die die Zeichnung, die Merkmale, die Ballons und das Protokoll aller
Änderungen enthält. Danach speichert ⌘S in diese Datei. **Speichern unter** (⇧⌘S) speichert unter
einem anderen Namen.

Zwischen dem Speichern schreibt Dimo jede Änderung in ein Sicherungsjournal. Nach einem Absturz oder
Stromausfall stellt das erneute Öffnen des Projekts (oder der Start von Dimo, bei einem nie
gespeicherten Projekt) die Arbeit wieder her, mit einem Hinweis über der Zeichnung. Beim Schließen
des Fensters oder beim Anlegen oder Öffnen eines anderen Projekts mit ungespeicherten Änderungen
fragt Dimo zuerst nach. Einzelheiten: [Projekte](projects.md).

### 9. Exportieren

Öffne die Ansicht **Export** (⌘E).

- **PDF mit Ballons**: die Zeichnung mit eingezeichneten Ballons. Die Originaldatei bleibt
  unverändert.
- **Merkmalsliste (CSV)** und **(Excel)**: eine Zeile pro Merkmal, in Ballonreihenfolge.

Wähle die Sprache der Spaltenüberschriften und wie das PDF die Ballons enthält. Jeder Export fragt,
wo die Datei gespeichert wird, und läuft im Hintergrund. Setze für eine Zeichnung, die an einen
Kunden geht, vorher das Häkchen bei **Als freigegeben exportieren**: Es sperrt die Nummerierung,
die Nummern ändern sich danach nie mehr, auch wenn du Merkmale hinzufügst oder löschst.
Einzelheiten: [Exporte](exports.md).

### 10. Einstellungen

Unter **Einstellungen** (⌘,) wählst du das Farbschema (hell, dunkel oder wie das System), die
Sprache der App und den Benutzernamen, der in das Änderungsprotokoll jedes Projekts geschrieben
wird. Die Einstellungen bleiben für den nächsten Start erhalten. Einzelheiten:
[Einstellungen](settings.md).

### Wie es weitergeht

- Öffne das Projekt später mit **Projekt öffnen** (⌘O) und mache weiter. Jede Änderung, die du
  gemacht hast, steht im Änderungsprotokoll im Projekt.
- Noch nicht in dieser Version: automatisches Durchsuchen der Zeichnung, Wahl einer
  Allgemeintoleranz, Prüfergebnisse und Berichte. Die Ansichten **Prüfen** und **Messen** zeigen einen
  Hinweis, dass sie in einer späteren Version folgen.
