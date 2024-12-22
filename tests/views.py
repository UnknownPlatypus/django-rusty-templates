from django.shortcuts import render


def home(request):
    return render(request, "templates/home.txt")


def bio(request, username):
    return render(request, "templates/bio.txt", {"username": username})
