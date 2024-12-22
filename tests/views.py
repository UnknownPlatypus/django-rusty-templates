from django.shortcuts import render


def home(request):
    return render(request, "templates/home.txt")  # pragma: no cover


def bio(request, username):
    return render(
        request, "templates/bio.txt", {"username": username}
    )  # pragma: no cover
