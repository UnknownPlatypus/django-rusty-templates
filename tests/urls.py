from django.urls import path

from . import views

urlpatterns = [
    path("", views.home, name="home"),
    path("bio/<username>/", views.bio, name="bio"),
]
