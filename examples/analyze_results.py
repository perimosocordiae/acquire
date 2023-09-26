#!/usr/bin/env python3
from typing import Sequence
import sys
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import scipy.stats as st


def main():
    scores = pd.read_csv(sys.stdin, header=None)
    num_games, num_players = scores.shape
    print("Per game scoring")
    print(scores.describe())

    _, axes = plt.subplots(ncols=3, figsize=(15, 5))
    axes[0].violinplot(scores, showextrema=False, widths=0.8)
    scores.plot(kind="box", ax=axes[0], xlabel="AI Player", ylabel="Score")
    axes[0].set_xlabel("AI Player")

    # compute rankings, see https://www.berkayantmen.com/rank
    ranks = scores.copy()
    ranks[:] = num_players - scores.values.argsort().argsort()
    print("\nPer game ranking (1.0 = winner)")
    print(ranks.describe())
 
    agent_names = scores.columns
    expected = num_games // num_players
    n = num_players + 1
    hist = pd.DataFrame(
        {
            name: np.bincount(ranks[name], minlength=n)[1:] - expected
            for name in agent_names
        },
        index=np.arange(1, n),
    )
    axes[1].axhline(expected, c="k", ls="--")
    hist.plot(
        kind="bar",
        ax=axes[1],
        legend=True,
        xlabel="Rank",
        ylabel="Frequency",
        bottom=expected,
    )

    # compute p-values for test (0) outperforming the others
    p_values = np.array(
        [
            st.wilcoxon(
                scores[agent_names[0]] - scores[name],
                zero_method="pratt",
                alternative="greater",
                mode="approx",
            ).pvalue
            for name in agent_names[1:]
        ]
    )
    print("\np-Values:", p_values)
    if (p_values < 0.05).all():
        print("Test condition is an improvement!")
    elif (p_values > 0.95).all():
        print("Test condition is a regression!")
    else:
        print("Test condition is inconclusive.")

    # compute running Elo ratings
    k = 20
    elos = np.zeros((len(scores) + 1, num_players))
    elos[0] = 1500 + np.arange(num_players)
    for i, s in enumerate(scores.values):
        elos[i + 1] = elos[i] + update_elos(elos[i], s.argsort(), k)
        k = max(0.99 * k, 1)
    print("\nFinal Elo ratings:")
    for elo, name in sorted(zip(elos[-1], agent_names), reverse=True):
        print(f"{name}: {elo:.2f}")

    lines = axes[2].plot(elos)
    axes[2].legend(lines, agent_names)
    axes[2].set_xlabel("Games Played")
    axes[2].set_ylabel("Elo rating")
    plt.tight_layout()
    plt.show()



def _elo_change(rating_loser: float, rating_winner: float, k: float):
    gap = rating_winner - rating_loser
    return k / (1 + 10 ** (gap / 400))


def update_elos(
    current_elos: Sequence[float], ranking: Sequence[int], elo_k: float
) -> list[float]:
    num_players = len(current_elos)
    elo_change = [0.0] * num_players
    for j in range(1, num_players):
        idx_loser = ranking[j - 1]
        idx_winner = ranking[j]
        delta = _elo_change(current_elos[idx_loser], current_elos[idx_winner], elo_k)
        elo_change[idx_loser] -= delta
        elo_change[idx_winner] += delta
    return elo_change


if __name__ == "__main__":
    main()
