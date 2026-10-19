# Zeichnungen aus Fotos und Scans

Wenn du nur eine Papierzeichnung hast, fotografiere sie mit dem Telefon oder scanne sie und
balloniere das Bild. DimoMark macht aus dem Bild ein gerades Blatt in seinem Papierformat; du
setzt die Ballons und tippst die Werte ein, wie bei einem PDF ohne Text.

## Ein gutes Foto machen

Ein paar Sekunden Sorgfalt an der Kamera sparen Zeit in **Bild anpassen** und geben schärfere
Ballons:

- **Flaches Blatt:** Leg die Zeichnung auf einen Tisch und streich Knicke glatt. Beschwere
  gewellte Ecken. Ein gebogenes Blatt lässt sich nachher nicht gerade machen.
- **Gleichmäßiges Licht:** Tageslicht oder Deckenlicht, kein Blitz. Vermeide den Schatten des
  Telefons und deiner Hand, und dreh das Blatt so, dass sich kein Licht darauf spiegelt.
- **Ganzes Blatt im Bild:** alle vier Ecken des Blatts mit etwas Tisch drumherum. DimoMark findet
  den Blattrand für die Eckenkorrektur nur, wenn es ihn sieht.
- **Von gerade oben:** Halte das Telefon parallel zum Blatt, über seiner Mitte. Eine kleine
  Schräglage korrigieren die Ecken, eine starke lässt kleine Schrift am fernen Ende unscharf.
- **Scharf:** Tippe zum Scharfstellen auf das Blatt im Bildschirm, halte still und vergrößere das
  Foto danach, um zu prüfen, ob die kleinste Toleranz lesbar ist.
- **Kein HEIC:** iPhones speichern Fotos standardmäßig als HEIC, das DimoMark nicht öffnen kann.
  Wähle Einstellungen > Kamera > Formate > Maximale Kompatibilität, oder sende dir das Foto per
  Mail oder Messenger, das macht meist ein JPEG daraus.
- **Scans:** 300 dpi in Graustufen oder Schwarzweiß reichen. Speichere als PNG oder TIFF; ein TIFF
  mit mehreren Seiten ergibt ein Blatt pro Seite.

## Ein Foto oder einen Scan öffnen

Ein Foto oder Scan einer Papierzeichnung öffnet sich wie ein PDF: wähle die JPEG-, PNG- oder
TIFF-Datei unter **Neues Projekt**. Ein TIFF mit mehreren Seiten ergibt ein Blatt pro Seite.
DimoMark dreht ein Foto so, wie das Telefon es aufgenommen hat, und legt das Bild unverändert im
Projekt ab.

- **Blattgröße:** Ein Scan, der seine Auflösung angibt (100 bis 1200 dpi), bekommt seine echte
  Größe. Ein Foto kennt seine Größe nicht, darum wird seine lange Seite auf 297 mm gesetzt, die
  Länge eines A4-Blatts. Große Bilder werden bei dieser Größe mit höchstens 300 dpi gehalten.
- **Kein Text im Bild:** Ein Rahmen um ein Maß findet keinen Text, also wird der Ballon gesetzt und
  du tippst den Wert ein, wie bei einem PDF ohne Text.
- **HEIC-Fotos** können nicht geöffnet werden, siehe die Tipps oben.
- Merkmalsliste, Berichte und das PDF mit Ballons funktionieren wie bei einem PDF. Das PDF mit
  Ballons zeigt das angepasste Blatt in seinem Papierformat ([Exporte](exports.md)).

## Bild anpassen

**Zeichnung > Bild anpassen** (auch der erste Schritt unter **Zeichnung einrichten** nach dem Öffnen eines
Fotos) macht aus einem Foto ein gerades Blatt. Das Originalbild bleibt unverändert im Projekt; die
Einstellungen lassen sich jederzeit wieder ändern.

1. **Drehen:** in 90-Grad-Schritten drehen, dann mit dem Schieber gerade richten (bis 10 Grad in
   beide Richtungen) oder einen Winkel eintippen (bis 45 Grad; positiv dreht im Uhrzeigersinn).
2. **Ecken:** Bei einem schräg aufgenommenen Foto schalte "Ecken korrigieren" ein. DimoMark sucht
   den Blattrand und setzt die vier Griffe auf seine Ecken; findet es ihn nicht, beginnen die
   Griffe an den Bildecken. Zieh sie auf die Ecken des Blatts. Die Ecken richten das Blatt selbst
   gerade, darum wird der Winkel auf 0 gesetzt.
3. **Zuschneiden:** Zieh die Ecken des Rechtecks oder das Rechteck selbst, damit nur das Blatt
   bleibt.
4. **Papier:** Wähle das Papier, das das Foto zeigt (A4 bis A0, Letter, Tabloid oder eine eigene
   Größe in mm) und Hoch- oder Querformat. Das zugeschnittene Bild kommt auf dieses Blatt, damit
   gemessene Maße passen. "Wie im Foto" behält die Größe des Fotos.
5. **Aufbereiten** (freiwillig) zeigt das Bild grau mit weißem Hintergrund und kräftigeren Linien.

**Übernehmen** ist ein Schritt, den **Rückgängig** zurücknimmt. Bereits gesetzte Ballons bleiben
an ihrer Stelle auf der Zeichnung. Würden Ballons außerhalb des neuen Blatts liegen, fragt DimoMark
zuerst; "Trotzdem übernehmen" setzt sie an den Blattrand. **Zurücksetzen** stellt das Bild wieder
so her, wie es geöffnet wurde.
