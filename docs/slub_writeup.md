# SLAB vs SLUB — notes de synthèse

Ce document résume les concepts clés des allocateurs **SLAB** et **SLUB** de Linux, avec un focus sur leurs mécanismes et implications sécurité.

## Concepts de base

- **kmem_cache** : une structure (cache) qui gère des objets d'une taille fixe.
- **slab** : unité de mémoire (souvent une ou plusieurs pages) découpée en objets du cache.
- **freelist** : liste chaînée intrusive des objets libres.
- **per-cpu cache** : mini-cache par CPU pour réduire la contention.

## SLAB (historique)

- Utilise des **slabs** contenant des objets initialisés et des métadonnées séparées.
- Offre des états : *full*, *partial*, *empty*.
- Bonne performance mais métadonnées plus lourdes.

## SLUB (Simplified SLAB)

- Simplifie la gestion en stockant plus de métadonnées **dans la slab**.
- Élimine une partie des structures SLAB traditionnelles.
- Favorise les allocations rapides, surtout pour les petits objets.

## Structures clés (Linux)

- `kmem_cache` : décrit la taille, l'alignement, les constructeurs/destructeurs.
- **per-cpu freelist** : fast path sans locks.
- **partial slabs** : slabs partiellement utilisées, liste globale ou par CPU.

## Comparaison rapide

| Aspect | SLAB | SLUB |
|---|---|---|
| Complexité | Plus élevée | Plus simple |
| Métadonnées | Externes | Internes |
| Perfs | Correctes | Souvent meilleures |
| Debug | Outils classiques | SLUB debug hooks |

## Sécurité / exploitation

- **UAF (Use-After-Free)** : un objet libéré puis réutilisé peut être réécrit.
- **Double free** : corruption de la freelist, potentiellement arbitrage d'écriture.
- **Overflow** : dépassement de tampon vers l'objet suivant dans la slab.
- **Mitigations** : redzones, freelist hardening, quarantine, poisoning.

## Glossaire rapide

- **Object size** : taille d'allocation fixe du cache.
- **Slab page** : page utilisée comme slab (ex: 4096 bytes).
- **Free list** : liste des objets libres, utilisée pour O(1).
