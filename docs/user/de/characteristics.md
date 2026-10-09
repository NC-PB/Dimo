# Merkmalstabelle

Die Tabelle unter der Zeichnung listet alle Merkmale des Projekts in der Reihenfolge der
Ballonnummern. Ziehe die Linie über der Tabelle, um ihre Höhe zu ändern (oder fokussiere sie und
nutze ↑ und ↓).

## Spalten

| Spalte | Inhalt |
|---|---|
| Nr | Ballonnummer. Ziehe den Griff ⠿, um das Merkmal zu verschieben |
| Art | Längenmaß, Durchmesser, Radius, Kugelradius, Winkel, Fase, Gewinde, Flachsenkung, Kegelsenkung, Tiefe, Oberflächenbeschaffenheit, geometrische Toleranz, Hinweis, Fahnenhinweis, Werkstoff oder Verfahren, Sonstiges |
| Anforderung | Text wie auf der Zeichnung |
| Nennmaß | Nennwert |
| Oberes Abmaß, Unteres Abmaß | Abmaße mit Vorzeichen |
| Obere Grenze, Untere Grenze | Grenzwerte. Dimo berechnet sie aus Nennmaß und Abmaßen; du kannst sie auch eintippen |
| Einheit | mm, in, ° oder keine. Ein Nennmaß ohne Einheit erhält die Einheit des Blatts (Grad bei Winkeln) |
| Passung | Passungsangabe wie `H7` |
| Anzahl | Anzahl der Elemente, für die das Merkmal steht, mindestens 1 |
| Klasse | Kritisch, Hauptmerkmal, Nebenmerkmal, Schlüsselmerkmal, oder leer, wenn nicht klassifiziert |
| Prüfmethode, Prüfmittel, Stichprobe, Häufigkeit | Wie das Merkmal geprüft wird, freier Text |
| Kommentar | Freier Text |
| Prüfen | Leer bei Hilfs- und theoretisch genauen Maßen |

Zahlen erscheinen mit genau den gespeicherten Stellen, `90.0` bleibt `90.0`.

## Auswählen

Eine ausgewählte Zeile legt einen dicken Ring um ihren Ballon auf der Zeichnung, und ein Klick auf
einen Ballon wählt seine Zeile aus und rollt sie ins Bild. Liegt der Ballon auf einem anderen
Blatt, wird dieses Blatt mit dem Ballon in der Mitte gezeigt; ein Ballon außerhalb des sichtbaren
Teils der Zeichnung wird ebenfalls in die Mitte geholt.

- Klicke eine Zeile, um sie auszuwählen. ⌘ Klick (Strg Klick) fügt eine Zeile hinzu oder entfernt
  sie, Umschalt Klick wählt einen Bereich.
- ↑ und ↓ gehen durch die Zeilen, Umschalt ↑ und Umschalt ↓ erweitern die Auswahl, ⌘A (Strg+A)
  wählt alle.

## Bearbeiten

Doppelklicke eine Zelle, drücke Eingabe oder F2, oder beginne zu tippen. Eingabe bestätigt,
Escape bricht ab, Tab bestätigt und geht zur nächsten Spalte, ↑ und ↓ bestätigen und gehen zur
Zeile darüber oder darunter. Art, Einheit und Klasse öffnen eine Liste. Leertaste schaltet Prüfen
um.

Tippe Zahlen mit Punkt als Dezimaltrennzeichen, zum Beispiel `-0.05`. Kann Dimo einen Wert nicht
verwenden, bleibt die Zelle offen und die Zeile über der Tabelle sagt warum. Jede Änderung lässt
sich mit ⌘Z (Strg+Z) rückgängig machen.

## Reihenfolge ändern

Ziehe den Griff ⠿ in der Spalte Nr, oder wähle Zeilen aus und drücke ⌥↑ oder ⌥↓ (Alt+↑ oder
Alt+↓). Mehrere ausgewählte Zeilen bewegen sich zusammen und behalten ihre Reihenfolge. Alle
Merkmale werden im selben Schritt neu nummeriert, ein Rückgängig stellt alles wieder her.

Solange die Nummerierung gesperrt ist, zum Beispiel nach dem Ausgeben eines Berichts, lässt sich
die Reihenfolge nicht ändern. Die Zeile über der Tabelle zeigt, wer die Nummerierung wann gesperrt
hat.

## Tastatur

Die Liste der Tastenkürzel (`?`) hat einen Abschnitt für die Tabelle. Solange die Tabelle den
Fokus hat, wirken Pfeiltasten, Eingabe, Escape, Leertaste und ⌘A (Strg+A) in der Tabelle, und
einzelne Buchstabentasten wie R, B oder S wirken nie auf die Zeichnung. Kürzel mit ⌘ (Strg), zum
Beispiel Sichern und Rückgängig, wirken überall.
