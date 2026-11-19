"""
Revisa un .obj exportado de Blender ANTES de abrir el juego.

Uso (desde la raiz del repo):
    python tools/check_obj.py assets/models/ship.obj
    python tools/check_obj.py assets/models/*.obj

Dice: triangulos, tamano (caja), si el origen esta bien, si los materiales existen
en el .mtl y si la ruta del mtllib es portable.
"""
import math
import os
import sys

# Rangos esperados por el juego (tamano de la caja que rodea al modelo).
# nombre: (ancho X, alto Y, largo Z, max triangulos, donde debe estar el origen)
ESPERADO = {
    "ship": ((2.0, 5.0), (0.8, 2.5), (2.5, 5.0), 1500, "centro"),
    "tree": ((0.6, 1.4), (1.4, 2.2), (0.6, 1.4), 400, "base"),
    "moto": ((0.2, 1.0), (0.8, 1.6), (1.4, 2.4), 1200, "base"),
    "goal": ((1.6, 2.6), (0.8, 1.4), (0.5, 1.2), 800, "base"),
    "kiss": ((0.8, 1.4), (1.0, 1.7), (0.8, 1.4), 400, "base"),
}


def revisar(path):
    nombre = os.path.splitext(os.path.basename(path))[0]
    verts, tris, usados, mtllib = [], 0, [], None
    with open(path, encoding="utf-8", errors="replace") as f:
        for linea in f:
            p = linea.split()
            if not p:
                continue
            if p[0] == "v":
                verts.append(tuple(map(float, p[1:4])))
            elif p[0] == "f":
                tris += max(0, len(p) - 3)          # n lados -> n-2 triangulos
            elif p[0] == "usemtl" and p[1] not in usados:
                usados.append(p[1])
            elif p[0] == "mtllib":
                mtllib = p[1]
    print(f"\n=== {path} ===")
    if not verts or tris == 0:
        print("  ERROR: no tiene vertices o triangulos")
        return
    xs, ys, zs = zip(*verts)
    dx, dy, dz = max(xs) - min(xs), max(ys) - min(ys), max(zs) - min(zs)
    radio = max(math.sqrt(x*x + y*y + z*z) for x, y, z in verts)
    print(f"  vertices: {len(verts)}   triangulos: {tris}")
    print(f"  tamano X={dx:.2f}  Y={dy:.2f}  Z={dz:.2f}   (radio desde el origen: {radio:.2f})")
    print(f"  X: {min(xs):.2f}..{max(xs):.2f}   Y: {min(ys):.2f}..{max(ys):.2f}   Z: {min(zs):.2f}..{max(zs):.2f}")

    # --- materiales
    if mtllib is None:
        print("  AVISO: no hay 'mtllib' (el modelo saldra gris). Activa Materials al exportar.")
    else:
        if os.path.isabs(mtllib) or "\\" in mtllib or "/" in mtllib:
            print(f"  AVISO: mtllib tiene una ruta ({mtllib}). Exporta con Path Mode = Strip "
                  "para que funcione en otra computadora.")
        mtl_path = os.path.join(os.path.dirname(path), os.path.basename(mtllib.replace("\\", "/")))
        if not os.path.exists(mtl_path):
            print(f"  ERROR: no existe {mtl_path}")
        else:
            definidos = set()
            with open(mtl_path, encoding="utf-8", errors="replace") as f:
                for linea in f:
                    p = linea.split()
                    if p and p[0] == "newmtl":
                        definidos.add(p[1])
            faltan = [m for m in usados if m not in definidos]
            print(f"  materiales usados: {', '.join(usados) or '(ninguno)'}")
            if faltan:
                print(f"  ERROR: materiales sin definir en el .mtl: {faltan}")

    # --- comparacion con lo que espera el juego
    if nombre in ESPERADO:
        (ax, ay, az, maxt, origen) = ESPERADO[nombre]
        for etiqueta, valor, (lo, hi) in (("X", dx, ax), ("Y", dy, ay), ("Z", dz, az)):
            if not lo <= valor <= hi:
                print(f"  AVISO: el tamano en {etiqueta} es {valor:.2f}; el juego espera entre {lo} y {hi}. "
                      "Revisa la escala o los ejes (Forward=+Z, Up=+Y).")
        if tris > maxt:
            print(f"  AVISO: {tris} triangulos; conviene menos de {maxt} (se dibuja muchas veces).")
        if origen == "base" and abs(min(ys)) > 0.15:
            print(f"  AVISO: el origen debe estar en la BASE (Y minimo = {min(ys):.2f}, deberia ser ~0).")
        if origen == "centro" and radio > 3.2:
            print(f"  AVISO: el origen deberia estar en el centro de la nave (radio {radio:.2f} es muy grande).")
    print("  Revision terminada.")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    for ruta in sys.argv[1:]:
        revisar(ruta)