# Merkmalstabelle

Die Tabelle unter der Zeichnung listet alle Merkmale des Projekts in der Reihenfolge der
Ballonnummern. Ziehe die Linie über der Tabelle, um ihre Höhe zu ändern (oder fokussiere sie und
nutze ↑ und ↓).

## Spalten

| Spalte | Inhalt |
|---|---|
| Nr | Ballonnummer. Ziehe den Griff ⠿, um das Merkmal zu verschieben |
| Art | Längenmaß, Durchmesser, Radius, Kugelradius, Winkel, Fase, Gewinde, Flachsenkung, Kegelsenkung, Tiefe, Oberflächenbeschaffenheit, geometrische Toleranz, Hinweis, Fahnenhinweis, Werkstoff oder Verfahren, Sonstiges. Ein von dir gesetzter Ballon beginnt mit Sonstiges |
| Anforderung | Der Text, den du in den Ballon getippt hast, wie auf der Zeichnung |
| Nennmaß | Nennwert |
| Oberes Abmaß, Unteres Abmaß | Abmaße mit Vorzeichen |
| Obere Grenze, Untere Grenze | Grenzwerte. Dimo berechnet sie aus Nennmaß und Abmaßen; du kannst sie auch eintippen |
| Einheit | mm, in, ° oder keine. Ein Nennmaß ohne Einheit erhält die Einheit des Blatts (Grad bei Winkeln) |
| Passung | Passungsangabe wie `H7` |
| Anzahl | Anzahl der Elemente, für die das Merkmal steht, mindestens 1 |
| Klasse | Kritisch, Hauptmerkmal, Nebenmerkmal, Schlüsselmerkmal, oder leer, wenn nicht klassifiziert |
| Prüfmethode, Prüfmittel, Stichprobe, Häufigkeit | Wie das Merkmal geprüft wird, freier Text |
| Kommentar | Freier Text |
| Prüfen | Häkchen weg bei Hilfsmaßen und theoretisch genauen Maßen, die nicht gemessen werden. Sie bleiben in der Liste und werden mit `Prüfen` gleich `no` exportiert |

Zahlen erscheinen mit genau den gespeicherten Stellen, `90.0` bleibt `90.0`.

## Auswählen

Eine ausgewählte Zeile legt einen dicken Ring um ihren Ballon auf der Zeichnung, und ein Klick auf
einen Ballon wählt seine Zeile aus und rollt sie ins Bild. Liegt der Ballon auf einem anderen
Blatt, wird dieses Blatt mit dem Ballon in der Mitte gezeigt; ein Ballon außerhalb des sichtbaren
Teils der Zeichnung wird ebenfalls in die Mitte geholt.

- Klicke eine Zeile, um sie auszuwählen. ⌘ Klick (Strg Klick) fügt eine Zeile hinzu oder entfernt
  sie, Shift Klick wählt einen Bereich.
- ↑ und ↓ gehen durch die Zeilen, Shift ↑ und Shift ↓ erweitern die Auswahl, ⌘A (Strg+A)
  wählt alle.

## Bearbeiten

Doppelklicke eine Zelle, drücke Eingabe oder F2, oder beginne zu tippen. Eingabe bestätigt,
Escape bricht ab, Tab bestätigt und geht zur nächsten Spalte, ↑ und ↓ bestätigen und gehen zur
Zeile darüber oder darunter. Art, Einheit und Klasse öffnen eine Liste. In der Spalte Prüfen
schaltet die Leertaste das Häkchen um.

Tippe Zahlen mit Punkt als Dezimaltrennzeichen, zum Beispiel `-0.05`. Kann Dimo einen Wert nicht
verwenden, bleibt die Zelle offen und die Zeile über der Tabelle sagt warum. Jede Änderung lässt
sich mit ⌘Z (Strg+Z) rückgängig machen.

## Grenzwerte

Dimo berechnet die Grenzwerte aus dem Nennmaß und beiden Abmaßen, mit dem Vorzeichen, das du
getippt hast: Nennmaß `8.0` mit oberem Abmaß `0.02` und unterem Abmaß `-0.05` ergibt die obere
Grenze `8.02` und die untere Grenze `7.95`. Änderst du das Nennmaß oder ein Abmaß, ziehen die
Grenzwerte nach. Entfernst du ein Abmaß, verschwinden auch die berechneten Grenzwerte. Einen
Grenzwert, den du selbst tippst, behält Dimo, und er geht den Abmaßen vor. Alle Zahlen sind exakte
Dezimalzahlen; nichts wird gerundet.

## Löschen

Die Tabelle hat keine Löschtaste. Wähle die Zeilen aus und verwende **Löschen** in der
Werkzeugleiste über der Zeichnung, oder wähle die Ballons auf der Zeichnung aus und drücke
**Entfernen**. Die Ballons werden mit ihren Merkmalen gelöscht, und ⌘Z holt alles zurück.

## Reihenfolge ändern

Ziehe den Griff ⠿ in der Spalte Nr, oder wähle Zeilen aus und drücke ⌥↑ oder ⌥↓ (Alt+↑ oder
Alt+↓). Mehrere ausgewählte Zeilen bewegen sich zusammen und behalten ihre Reihenfolge. Alle
Merkmale werden im selben Schritt neu nummeriert, ein Rückgängig stellt alles wieder her.

Solange die Nummerierung gesperrt ist, zum Beispiel nach dem Ausgeben eines Berichts, lässt sich
die Reihenfolge nicht ändern. Die Zeile über der Tabelle zeigt, wer die Nummerierung wann gesperrt
hat.

## Tastatur

Die Liste der Tastenkürzel (`?`) hat einen Abschnitt für die Tabelle; alle Tabellentasten stehen
auch unter [Tastenkürzel](shortcuts.md). Solange die Tabelle den Fokus hat, wirken Pfeiltasten,
Pos1, Ende, Eingabe, Escape, Leertaste und ⌘A (Strg+A) in der Tabelle, und einzelne Tasten wie R,
B, S, **+**, **-** oder **0** wirken nie auf die Zeichnung. Kürzel mit ⌘ (Strg), zum Beispiel
Speichern und Rückgängig, wirken überall. Ein Klick auf einen Ballon oder, im Werkzeug Auswählen,
auf leere Zeichnung gibt der Zeichnung den Tastaturfokus zurück.
