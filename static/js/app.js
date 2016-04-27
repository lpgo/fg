var app = angular.module('app',['ngRoute']);

app.controller('ListCtrl', ['$scope', function($scope){
	$scope.count = 15;
}])