# Projekt- und Berichtsdaten

Das Feld **Projekt und Bericht** in der Seitenleiste enthält die Daten, die Teil, Zeichnung und
Prüfbericht kennzeichnen. Es ist sichtbar, solange ein Projekt offen ist, unterhalb der
Blatteigenschaften. Die Daten liegen im Projekt und stehen in Kopfzeile und Unterschriftsblock der
Berichte.

Jedes Feld wird gespeichert, wenn du es verlässt oder Enter drückst. Jede Änderung ist ein Schritt
beim Rückgängigmachen (⌘Z), wie jede andere Änderung am Projekt. Lehnt Dimo einen Wert ab, zeigt das
Feld wieder den alten Wert, und eine Meldung über der Zeichnung nennt den Grund.

## Teil

| Feld | Bedeutung |
|---|---|
| Teilenummer | Nummer des Teils, wie der Kunde sie nennt |
| Teilebezeichnung | Name des Teils |
| Kunde | Wer das Teil bestellt hat |
| Auftragsnummer | Auftrags- oder Bestellbezug |

## Zeichnung

| Feld | Bedeutung |
|---|---|
| Revisionsstand | Revisionsstand der Zeichnung, zum Beispiel `B`. Dimo liest ihn nicht aus dem Schriftfeld, tippe ihn von der Zeichnung ab |
| Zeichnungsdatei | Dateiname der importierten Zeichnung (nicht bearbeitbar) |
| SHA-256 | Fingerabdruck der Zeichnungsdatei (nicht bearbeitbar). **Kopieren** legt ihn in die Zwischenablage. Der Bericht druckt ihn, so lässt sich prüfen, dass ein Bericht zu genau dieser Zeichnung gehört |

Der Revisionsstand gehört zur Zeichnung im Projekt. Dimo speichert ihn mit dem Projekt und druckt
ihn auf jeden Bericht.

## Bericht

| Feld | Bedeutung |
|---|---|
| Berichtsnummer | Nummer des Prüfberichts |
| Berichtsdatum | Datum des Berichts. **Heute** trägt einmalig das Datum deines Computers ein; das Datum steht dann im Projekt und ändert sich nicht von selbst |
| Firma | Name deiner Firma. Siehe unten |
| Geprüft von | Wer die Teile geprüft hat |
| Freigegeben von | Wer den Bericht freigegeben hat |
| Bemerkungen | Freier Text, der auf dem Bericht steht |

Leere Felder bleiben auf dem Bericht leer. Dimo setzt von sich aus kein Datum und keinen Namen ein.

### Firmenname

Trage deinen Firmennamen einmal in den **Einstellungen** unter **Berichte** ein
([Einstellungen](settings.md)). Öffnest du ein Projekt, dessen Berichtsdaten noch keinen Firmennamen
haben, kopiert Dimo den Namen aus den Einstellungen in das Feld. Das ist ein Schritt beim
Rückgängigmachen. Ein Name, der schon im Projekt steht, wird nie ersetzt, und eine spätere Änderung
der Einstellung ändert bestehende Projekte nicht.
